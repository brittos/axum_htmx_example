//! Service para lógica de RBAC (Role-Based Access Control).

use crate::models::view::{RbacAction, RbacPermission, RbacResource, RbacRole};
use entity::{
    actions, permission_actions, permissions, resources, role_permissions, roles, user_roles,
};
use redis::AsyncCommands;
use sea_orm::{ColumnTrait, DbConn, EntityTrait, QueryFilter, QueryOrder};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Dados completos do RBAC para renderização
pub struct RbacMatrix {
    pub active_role: String,
    pub roles: Vec<RbacRole>,
    pub resources: Vec<RbacResource>,
    pub actions: Vec<RbacAction>,
    pub permissions: HashMap<String, HashMap<String, RbacPermission>>,
}

pub struct RbacService;

impl RbacService {
    /// Constrói a matriz completa de permissões RBAC
    pub async fn build_permissions_matrix(
        db: &DbConn,
        requested_role: Option<String>,
    ) -> RbacMatrix {
        // 1. Fetch Basic Data
        let roles_db = roles::Entity::find()
            .order_by_asc(roles::Column::Name)
            .all(db)
            .await
            .unwrap_or_default();

        let resources_db = resources::Entity::find()
            .order_by_asc(resources::Column::Name)
            .all(db)
            .await
            .unwrap_or_default();

        let mut actions_db = actions::Entity::find().all(db).await.unwrap_or_default();

        // Custom logical sorting
        actions_db.sort_by_key(|a| match a.name.as_str() {
            "read" => 1,
            "create" => 2,
            "edit" => 3,
            "delete" => 4,
            "approve" => 5,
            "print" => 6,
            _ => 100,
        });

        // 2. Map to ViewModels
        let roles: Vec<RbacRole> = roles_db
            .iter()
            .map(|r| RbacRole {
                name: r.name.clone(),
                bg_color: Self::role_color(&r.name),
            })
            .collect();

        let resources_vm: Vec<RbacResource> = resources_db
            .iter()
            .map(|r| RbacResource {
                name: r.name.clone(),
                icon: Self::resource_icon(&r.name),
            })
            .collect();

        let actions_vm: Vec<RbacAction> = actions_db
            .iter()
            .map(|a| RbacAction {
                name: a.name.clone(),
            })
            .collect();

        // 3. Build Permissions Matrix
        let all_role_perms = role_permissions::Entity::find()
            .all(db)
            .await
            .unwrap_or_default();
        let all_perms = permissions::Entity::find()
            .all(db)
            .await
            .unwrap_or_default();
        let all_perm_actions = permission_actions::Entity::find()
            .all(db)
            .await
            .unwrap_or_default();

        // Structure helpers
        let mut perm_id_to_resource_id: HashMap<Uuid, Uuid> = HashMap::new();
        for p in &all_perms {
            perm_id_to_resource_id.insert(p.id, p.resource_id);
        }

        let mut role_id_to_perm_ids: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for rp in &all_role_perms {
            role_id_to_perm_ids
                .entry(rp.role_id)
                .or_default()
                .push(rp.permission_id);
        }

        let mut perm_id_to_actions: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for pa in &all_perm_actions {
            if pa.allowed {
                perm_id_to_actions
                    .entry(pa.permission_id)
                    .or_default()
                    .push(pa.action_id);
            }
        }

        let mut permissions_map = HashMap::new();

        for role in &roles_db {
            let mut res_map = HashMap::new();
            let role_perm_ids = role_id_to_perm_ids
                .get(&role.id)
                .cloned()
                .unwrap_or_default();

            for res in &resources_db {
                let target_perm_id = role_perm_ids
                    .iter()
                    .find(|pid| perm_id_to_resource_id.get(pid) == Some(&res.id));

                let mut acts_map = HashMap::new();

                if let Some(pid) = target_perm_id {
                    let allowed_action_ids =
                        perm_id_to_actions.get(pid).cloned().unwrap_or_default();
                    for act in &actions_db {
                        let is_allowed = allowed_action_ids.contains(&act.id);
                        acts_map.insert(act.name.clone(), is_allowed);
                    }
                } else {
                    for act in &actions_db {
                        acts_map.insert(act.name.clone(), false);
                    }
                }

                res_map.insert(res.name.clone(), RbacPermission { actions: acts_map });
            }
            permissions_map.insert(role.name.clone(), res_map);
        }

        // Active Role Logic
        let mut active_role = requested_role.unwrap_or_else(|| "Admin".to_string());
        if !roles.iter().any(|r| r.name == active_role) {
            active_role = roles.first().map(|r| r.name.clone()).unwrap_or(active_role);
        }

        RbacMatrix {
            active_role,
            roles,
            resources: resources_vm,
            actions: actions_vm,
            permissions: permissions_map,
        }
    }

    /// Retorna a cor do badge para uma role
    fn role_color(name: &str) -> String {
        crate::config::ui::role_color(name)
    }

    /// Retorna o ícone Lucide para um recurso
    fn resource_icon(name: &str) -> String {
        crate::config::ui::resource_icon(name)
    }
}

// ============================================================================
// Authorization Functions
// ============================================================================

/// Busca os nomes das roles do usuário.
pub async fn get_user_roles(db: &DbConn, user_id: Uuid) -> Vec<String> {
    let user_role_records = user_roles::Entity::find()
        .filter(user_roles::Column::UserId.eq(user_id))
        .all(db)
        .await
        .unwrap_or_default();

    let role_ids: Vec<Uuid> = user_role_records.iter().map(|ur| ur.role_id).collect();

    if role_ids.is_empty() {
        return Vec::new();
    }

    roles::Entity::find()
        .filter(roles::Column::Id.is_in(role_ids))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.name)
        .collect()
}

/// Busca todas as permissões do usuário baseado em suas roles.
/// Retorna HashMap<resource_name, HashSet<action_name>>.
pub async fn get_user_permissions(db: &DbConn, user_id: Uuid) -> HashMap<String, HashSet<String>> {
    // 1. Buscar role_ids do usuário
    let user_role_ids: Vec<Uuid> = user_roles::Entity::find()
        .filter(user_roles::Column::UserId.eq(user_id))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|ur| ur.role_id)
        .collect();

    if user_role_ids.is_empty() {
        return HashMap::new();
    }

    // 2. Buscar permission_ids das roles
    let role_perms = role_permissions::Entity::find()
        .filter(role_permissions::Column::RoleId.is_in(user_role_ids))
        .all(db)
        .await
        .unwrap_or_default();

    let perm_ids: Vec<Uuid> = role_perms.iter().map(|rp| rp.permission_id).collect();

    if perm_ids.is_empty() {
        return HashMap::new();
    }

    // 3. Buscar permissions com seus resources
    let perms_with_resources = permissions::Entity::find()
        .filter(permissions::Column::Id.is_in(perm_ids.clone()))
        .all(db)
        .await
        .unwrap_or_default();

    // 4. Buscar todos resources
    let all_resources = resources::Entity::find().all(db).await.unwrap_or_default();
    let resource_map: HashMap<Uuid, String> =
        all_resources.into_iter().map(|r| (r.id, r.name)).collect();

    // 5. Buscar permission_actions permitidas
    let allowed_perm_actions = permission_actions::Entity::find()
        .filter(permission_actions::Column::PermissionId.is_in(perm_ids))
        .filter(permission_actions::Column::Allowed.eq(true))
        .all(db)
        .await
        .unwrap_or_default();

    // 6. Buscar todas actions
    let all_actions = actions::Entity::find().all(db).await.unwrap_or_default();
    let action_map: HashMap<Uuid, String> =
        all_actions.into_iter().map(|a| (a.id, a.name)).collect();

    // 7. Montar mapeamento permission_id -> resource_name
    let perm_to_resource: HashMap<Uuid, String> = perms_with_resources
        .iter()
        .filter_map(|p| resource_map.get(&p.resource_id).map(|r| (p.id, r.clone())))
        .collect();

    // 8. Construir resultado final
    let mut result: HashMap<String, HashSet<String>> = HashMap::new();

    for pa in allowed_perm_actions {
        if let (Some(resource_name), Some(action_name)) = (
            perm_to_resource.get(&pa.permission_id),
            action_map.get(&pa.action_id),
        ) {
            result
                .entry(resource_name.clone())
                .or_default()
                .insert(action_name.clone());
        }
    }

    result
}

/// Busca permissões com cache Redis.
/// TTL de 5 minutos para evitar queries repetidas.
pub async fn get_user_permissions_cached(
    db: &DbConn,
    redis: &mut redis::aio::ConnectionManager,
    user_id: Uuid,
) -> HashMap<String, HashSet<String>> {
    let cache_key = format!("user:permissions:{}", user_id);

    // Tentar ler do cache
    if let Ok(cached) = redis.get::<_, String>(&cache_key).await
        && let Ok(perms) = serde_json::from_str::<HashMap<String, HashSet<String>>>(&cached)
    {
        tracing::debug!("Permission cache hit for user {}", user_id);
        return perms;
    }

    // Cache miss - buscar do banco
    let perms = get_user_permissions(db, user_id).await;

    // Salvar no cache (TTL 5 minutos)
    if let Ok(json) = serde_json::to_string(&perms) {
        let _: Result<(), _> = redis.set_ex(&cache_key, json, 300).await;
    }

    perms
}

/// Verifica se o usuário tem permissão para executar uma ação em um recurso.
///
/// Admin bypassa todas as verificações.
/// Usa cache Redis para evitar queries repetidas.
pub async fn check_permission(
    db: &DbConn,
    redis: &mut redis::aio::ConnectionManager,
    user_id: Uuid,
    resource: &str,
    action: &str,
) -> bool {
    // Admin bypass - verifica roles primeiro
    let roles = get_user_roles(db, user_id).await;
    if roles.iter().any(|r| r == "Admin") {
        tracing::debug!("Admin bypass for user {}", user_id);
        return true;
    }

    // Verificar permissões específicas
    let perms = get_user_permissions_cached(db, redis, user_id).await;

    let has_permission = perms
        .get(resource)
        .is_some_and(|actions| actions.contains(action));

    tracing::debug!(
        "Permission check: user={}, resource={}, action={}, result={}",
        user_id,
        resource,
        action,
        has_permission
    );

    has_permission
}

/// Invalida o cache de permissões de um usuário.
/// Deve ser chamado quando roles ou permissões são alteradas.
pub async fn invalidate_permission_cache(redis: &mut redis::aio::ConnectionManager, user_id: Uuid) {
    let perm_key = format!("user:permissions:{}", user_id);
    let sidebar_key = format!("user:sidebar:{}", user_id);

    // Invalidar permissões
    if let Err(e) = redis.del::<_, ()>(&perm_key).await {
        tracing::error!(
            "Failed to invalidate permission cache for {}: {}",
            user_id,
            e
        );
    }

    // Invalidar sidebar (que contem permissões cacheadas)
    if let Err(e) = redis.del::<_, ()>(&sidebar_key).await {
        tracing::error!("Failed to invalidate sidebar cache for {}: {}", user_id, e);
    } else {
        tracing::debug!("Invalidated permission and sidebar cache for {}", user_id);
    }
}

/// Invalida o cache de todos os usuários que possuem uma role específica.
pub async fn invalidate_role_cache(
    db: &DbConn,
    redis: &mut redis::aio::ConnectionManager,
    role_id: Uuid,
) {
    let user_ids: Vec<Uuid> = user_roles::Entity::find()
        .filter(user_roles::Column::RoleId.eq(role_id))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|ur| ur.user_id)
        .collect();

    if user_ids.is_empty() {
        return;
    }

    tracing::info!(
        "Invalidating cache for role {}: {} users affected",
        role_id,
        user_ids.len()
    );

    for uid in user_ids {
        invalidate_permission_cache(redis, uid).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_color_returns_string() {
        let color = RbacService::role_color("Admin");
        assert!(!color.is_empty());
    }

    #[test]
    fn test_role_color_different_roles() {
        let admin_color = RbacService::role_color("Admin");
        let user_color = RbacService::role_color("User");
        // Colors should be defined, even if same for unknown roles
        assert!(!admin_color.is_empty());
        assert!(!user_color.is_empty());
    }

    #[test]
    fn test_resource_icon_returns_string() {
        let icon = RbacService::resource_icon("users");
        assert!(!icon.is_empty());
    }

    #[test]
    fn test_resource_icon_unknown_returns_default() {
        let icon = RbacService::resource_icon("unknown_resource");
        // Should return some default icon
        assert!(!icon.is_empty());
    }
}
