//! Serviço de usuários com lógica de negócio.

use crate::dto::{CreateUserParams, UserUpdateParams};
use crate::error::AppError;
use crate::repository::{Mutation, Query};
use crate::utils::password::hash_password;
use ::entity::{roles, user_roles, users};
use sea_orm::{ActiveModelTrait, Set, TransactionTrait};
use sea_orm::{
    ColumnTrait, DbConn, DbErr, DeleteResult, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Struct para representar usuário com suas roles
#[derive(Clone, Debug)]
pub struct UserWithRole {
    pub user: users::Model,
    pub role_names: Vec<String>,
}

pub struct UserService;

impl UserService {
    pub async fn create_user(db: &DbConn, form_data: users::Model) -> Result<users::Model, DbErr> {
        Mutation::create_user(db, form_data).await
    }

    pub async fn update_user(
        db: &DbConn,
        id: Uuid,
        form_data: users::Model,
    ) -> Result<users::Model, DbErr> {
        Mutation::update_user_by_id(db, id, form_data).await
    }

    pub async fn delete_user(db: &DbConn, id: Uuid) -> Result<DeleteResult, DbErr> {
        Mutation::delete_user(db, id).await
    }

    pub async fn find_user_by_id(db: &DbConn, id: Uuid) -> Result<Option<users::Model>, DbErr> {
        Query::find_user_by_id(db, id).await
    }

    /// Busca usuários paginados com suas roles.
    ///
    /// Otimizado para evitar N+1 queries:
    /// - 1 query para contar total de usuários
    /// - 1 query para buscar usuários da página
    /// - 1 query para buscar TODAS as roles dos usuários retornados
    pub async fn find_users_with_roles_paginated(
        db: &DbConn,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<UserWithRole>, u64), DbErr> {
        use sea_orm::PaginatorTrait;

        // Query 1: Contar total
        let total = users::Entity::find().count(db).await?;
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u64;

        // Query 2: Buscar usuários da página
        let users_list = users::Entity::find()
            .order_by_asc(users::Column::Name)
            .offset((page.saturating_sub(1)) * per_page)
            .limit(per_page)
            .all(db)
            .await?;

        if users_list.is_empty() {
            return Ok((Vec::new(), total_pages.max(1)));
        }

        // Coletar IDs dos usuários
        let user_ids: Vec<Uuid> = users_list.iter().map(|u| u.id).collect();

        // Query 3: Buscar TODAS as roles de todos os usuários em uma única query
        let all_user_roles = user_roles::Entity::find()
            .filter(user_roles::Column::UserId.is_in(user_ids.clone()))
            .all(db)
            .await
            .unwrap_or_default();

        // Coletar IDs únicos das roles
        let role_ids: Vec<Uuid> = all_user_roles.iter().map(|ur| ur.role_id).collect();

        // Query 4 (opcional, apenas se houver roles): Buscar nomes das roles
        let roles_map: HashMap<Uuid, String> = if !role_ids.is_empty() {
            roles::Entity::find()
                .filter(roles::Column::Id.is_in(role_ids))
                .all(db)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|r| (r.id, r.name))
                .collect()
        } else {
            HashMap::new()
        };

        // Agrupar roles por user_id
        let mut user_roles_map: HashMap<Uuid, Vec<String>> = HashMap::new();
        for ur in all_user_roles {
            if let Some(role_name) = roles_map.get(&ur.role_id) {
                user_roles_map
                    .entry(ur.user_id)
                    .or_default()
                    .push(role_name.clone());
            }
        }

        // Montar resultado final
        let results: Vec<UserWithRole> = users_list
            .into_iter()
            .map(|user| {
                let role_names = user_roles_map
                    .remove(&user.id)
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| vec!["Sem Role".to_string()]);
                UserWithRole { user, role_names }
            })
            .collect();

        Ok((results, total_pages.max(1)))
    }

    /// Cria um usuário com roles em uma transação atômica.
    pub async fn create_user_with_roles(
        db: &DbConn,
        params: CreateUserParams,
    ) -> Result<users::Model, AppError> {
        let role_ids: Vec<Uuid> = params
            .role_ids
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        if role_ids.is_empty() {
            return Err(AppError::BadRequest(
                "Selecione pelo menos um perfil".to_string(),
            ));
        }

        let hashed_password = hash_password(&params.password).map_err(|e| {
            tracing::error!("Failed to hash password: {}", e);
            AppError::InternalServerError("Erro ao processar senha".to_string())
        })?;

        let txn = db.begin().await?;

        let user_id = Uuid::now_v7();
        let user_active = users::ActiveModel {
            id: Set(user_id),
            name: Set(params.name),
            username: Set(params.user),
            email: Set(params.email),
            password: Set(hashed_password),
            status: Set(params.status),
            is_banned: Set(false),
            avatar_url: Set(None),
            last_active: Set(None),
            created_at: Set(chrono::Utc::now().fixed_offset()),
            updated_at: Set(chrono::Utc::now().fixed_offset()),
        };

        let user = user_active.insert(&txn).await.map_err(|e| {
            tracing::error!("Failed to create user: {}", e);
            AppError::DbError(e)
        })?;

        for role_id in role_ids {
            let role_model = user_roles::ActiveModel {
                user_id: Set(user_id),
                role_id: Set(role_id),
            };
            role_model.insert(&txn).await.map_err(|e| {
                tracing::error!("Failed to assign role {}: {}", role_id, e);
                AppError::DbError(e)
            })?;
        }

        txn.commit().await?;

        Ok(user)
    }

    /// Atualiza um usuário e suas roles em uma transação atômica.
    pub async fn update_user_with_roles(
        db: &DbConn,
        id: Uuid,
        params: UserUpdateParams,
    ) -> Result<users::Model, AppError> {
        let role_ids: Vec<Uuid> = params
            .role_ids
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        if role_ids.is_empty() {
            return Err(AppError::BadRequest(
                "Selecione pelo menos um perfil".to_string(),
            ));
        }

        let txn = db.begin().await?;

        let existing = users::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::NotFound("Usuário não encontrado".to_string()))?;

        let mut active: users::ActiveModel = existing.into();
        active.name = Set(params.name);
        active.username = Set(params.user);
        active.email = Set(params.email);
        active.status = Set(params.status);
        active.updated_at = Set(chrono::Utc::now().fixed_offset());

        if let Some(password) = params.password.filter(|p| !p.is_empty()) {
            let hashed = hash_password(&password).map_err(|e| {
                tracing::error!("Failed to hash password: {}", e);
                AppError::InternalServerError("Erro ao processar senha".to_string())
            })?;
            active.password = Set(hashed);
        }

        let updated_user = active.update(&txn).await.map_err(|e| {
            tracing::error!("Failed to update user: {}", e);
            AppError::DbError(e)
        })?;

        // Atualizar roles: Delete all + Insert new
        user_roles::Entity::delete_many()
            .filter(user_roles::Column::UserId.eq(id))
            .exec(&txn)
            .await?;

        for role_id in role_ids {
            let role_model = user_roles::ActiveModel {
                user_id: Set(id),
                role_id: Set(role_id),
            };
            role_model.insert(&txn).await.map_err(|e| {
                tracing::error!("Failed to assign role {}: {}", role_id, e);
                AppError::DbError(e)
            })?;
        }

        txn.commit().await?;

        Ok(updated_user)
    }

    /// Deleta um usuário e suas roles em uma transação atômica.
    pub async fn delete_user_with_roles(db: &DbConn, id: Uuid) -> Result<(), AppError> {
        let txn = db.begin().await?;

        // 1. Deletar roles
        user_roles::Entity::delete_many()
            .filter(user_roles::Column::UserId.eq(id))
            .exec(&txn)
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete roles: {}", e);
                AppError::DbError(e)
            })?;

        // 2. Deletar usuário
        let user = users::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::NotFound("Usuário não encontrado".to_string()))?;

        let user_active: users::ActiveModel = user.into();
        user_active.delete(&txn).await.map_err(|e| {
            tracing::error!("Failed to delete user: {}", e);
            AppError::DbError(e)
        })?;

        txn.commit().await?;

        Ok(())
    }

    /// Busca um único usuário com suas roles pelo ID.
    pub async fn find_user_with_roles(
        db: &DbConn,
        id: Uuid,
    ) -> Result<Option<UserWithRole>, DbErr> {
        let user = match users::Entity::find_by_id(id).one(db).await? {
            Some(u) => u,
            None => return Ok(None),
        };

        // Buscar roles
        let user_roles = user_roles::Entity::find()
            .filter(user_roles::Column::UserId.eq(user.id))
            .all(db)
            .await?;

        let role_ids: Vec<Uuid> = user_roles.iter().map(|ur| ur.role_id).collect();

        let role_names = if !role_ids.is_empty() {
            roles::Entity::find()
                .filter(roles::Column::Id.is_in(role_ids))
                .all(db)
                .await?
                .into_iter()
                .map(|r| r.name)
                .collect()
        } else {
            Vec::new()
        };

        Ok(Some(UserWithRole { user, role_names }))
    }
}
