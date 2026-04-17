//! Handlers de RBAC (Role-Based Access Control).

use crate::dto::{RbacParams, RbacToggleParams};
use crate::handlers::admin_templates::{RbacPartialTemplate, UserManagementContentTemplate};
use crate::handlers::response::HtmlTemplate;
use crate::require_permission;
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use tower_cookies::Cookies;

/// Handler para partial de RBAC (HTMX)
pub async fn rbac_partial_handler(
    State(state): State<AppState>,
    Query(params): Query<RbacParams>,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::service::rbac_service::RbacService;

    // Verificar permissão: User Management - edit (RBAC é sub-recurso)
    let _user_id = require_permission!(state, cookies, "User Management", "edit");

    let matrix = RbacService::build_permissions_matrix(&state.conn, params.role).await;
    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    let mut res = HtmlTemplate(UserManagementContentTemplate {
        active_tab: "rbac".into(),
        users: None,
        rbac: Some(RbacPartialTemplate {
            active_role: matrix.active_role,
            roles: matrix.roles,
            resources: matrix.resources,
            actions: matrix.actions,
            permissions: matrix.permissions,
        }),
        page: 1,
        total_pages: 1,
        csrf_token,
    })
    .into_response();

    res.headers_mut()
        .insert("HX-Title", "Bero Admin | RBAC".parse().unwrap());
    Ok(res)
}

/// Handler para toggle de permissão RBAC (HTMX PATCH)
pub async fn rbac_toggle_handler(
    State(state): State<AppState>,
    cookies: Cookies,
    axum::Form(params): axum::Form<RbacToggleParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::service::rbac_service::invalidate_role_cache;
    use entity::{actions, permission_actions, permissions, resources, role_permissions, roles};
    use sea_orm::prelude::Uuid;

    use sea_orm::{
        ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, PaginatorTrait, QueryFilter, Set,
    };

    // Verificar permissão: User Management - edit (RBAC é sub-recurso)
    let _user_id = require_permission!(state, cookies, "User Management", "edit");

    // 1. Resolve IDs
    let role = roles::Entity::find()
        .filter(roles::Column::Name.eq(&params.role))
        .one(&state.conn)
        .await
        .unwrap_or_default()
        .expect("Role not found");
    let resource = resources::Entity::find()
        .filter(resources::Column::Name.eq(&params.resource))
        .one(&state.conn)
        .await
        .unwrap_or_default()
        .expect("Resource not found");
    let action = actions::Entity::find()
        .filter(actions::Column::Name.eq(&params.action))
        .one(&state.conn)
        .await
        .unwrap_or_default()
        .expect("Action not found");

    // 2. Find Existing Permission Logic
    let role_perms = role_permissions::Entity::find()
        .filter(role_permissions::Column::RoleId.eq(role.id))
        .all(&state.conn)
        .await
        .unwrap_or_default();

    let mut target_perm_id = None;
    for rp in role_perms {
        let p = permissions::Entity::find_by_id(rp.permission_id)
            .one(&state.conn)
            .await
            .unwrap_or_default();
        if let Some(perm) = p.filter(|perm| perm.resource_id == resource.id) {
            target_perm_id = Some(perm.id);
            break;
        }
    }

    let perm_id_to_edit = if let Some(pid) = target_perm_id {
        let count = role_permissions::Entity::find()
            .filter(role_permissions::Column::PermissionId.eq(pid))
            .count(&state.conn)
            .await
            .unwrap_or(0);
        if count > 1 {
            let new_p = permissions::ActiveModel {
                id: Set(Uuid::now_v7()),
                resource_id: Set(resource.id),
            }
            .insert(&state.conn)
            .await
            .expect("Failed to create permission");
            let old_actions = permission_actions::Entity::find()
                .filter(permission_actions::Column::PermissionId.eq(pid))
                .all(&state.conn)
                .await
                .unwrap_or_default();
            for oa in old_actions {
                permission_actions::ActiveModel {
                    permission_id: Set(new_p.id),
                    action_id: Set(oa.action_id),
                    allowed: Set(oa.allowed),
                }
                .insert(&state.conn)
                .await
                .ok();
            }
            if let Some(ol) = role_permissions::Entity::find_by_id((role.id, pid))
                .one(&state.conn)
                .await
                .unwrap_or_default()
            {
                let _ = ol.delete(&state.conn).await;
            }
            role_permissions::ActiveModel {
                role_id: Set(role.id),
                permission_id: Set(new_p.id),
            }
            .insert(&state.conn)
            .await
            .expect("Failed to link role");
            new_p.id
        } else {
            pid
        }
    } else {
        let new_p = permissions::ActiveModel {
            id: Set(Uuid::now_v7()),
            resource_id: Set(resource.id),
        }
        .insert(&state.conn)
        .await
        .expect("Failed to create permission");
        role_permissions::ActiveModel {
            role_id: Set(role.id),
            permission_id: Set(new_p.id),
        }
        .insert(&state.conn)
        .await
        .expect("Failed to link role");
        new_p.id
    };

    // 3. Update Action Status
    let is_granted_now = params.current_status == "granted";
    let new_allowed_state = !is_granted_now;

    let pa_opt = permission_actions::Entity::find()
        .filter(permission_actions::Column::PermissionId.eq(perm_id_to_edit))
        .filter(permission_actions::Column::ActionId.eq(action.id))
        .one(&state.conn)
        .await
        .unwrap_or_default();

    if let Some(pa) = pa_opt {
        let mut pa_active: permission_actions::ActiveModel = pa.into();
        pa_active.allowed = Set(new_allowed_state);
        pa_active.update(&state.conn).await.ok();
    } else {
        permission_actions::ActiveModel {
            permission_id: Set(perm_id_to_edit),
            action_id: Set(action.id),
            allowed: Set(new_allowed_state),
        }
        .insert(&state.conn)
        .await
        .ok();
    }

    // INVALIDATE CACHE for all users with this role
    // This ensures changes appear immediately for users
    invalidate_role_cache(&state.conn, &mut state.redis.clone(), role.id).await;

    // Render Button via Template
    let new_status_str = if new_allowed_state {
        "granted"
    } else {
        "denied"
    };

    use crate::handlers::admin_templates::RbacToggleButtonTemplate;
    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    let button_html = RbacToggleButtonTemplate {
        role: params.role.clone(),
        resource: params.resource.clone(),
        action: params.action.clone(),
        status: new_status_str.to_string(),
        is_granted: new_allowed_state,
        csrf_token,
    }
    .render()
    .unwrap_or_default();

    Ok(Html(button_html))
}
