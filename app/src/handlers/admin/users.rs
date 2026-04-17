//! Handlers de gerenciamento de usuários.

use crate::dto::UsersPageQuery;
use crate::handlers::admin_templates::{
    UserEditPartialTemplate, UserFormPartialTemplate, UserManagementContentTemplate,
    UserManagementFullTemplate, UsersTableTemplate,
};
use crate::handlers::response::{HtmlTemplate, render_page};
use crate::middleware::{get_sidebar_user, invalidate_sidebar_cache};
use crate::models::view::UserViewModel;
use crate::require_permission;
use crate::state::AppState;
use axum::extract::ConnectInfo;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use std::net::SocketAddr;
use tower_cookies::Cookies;

/// Handler para página de gerenciamento de usuários
pub async fn admin_users(
    state: State<AppState>,
    headers: HeaderMap,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::service::user_service::UserService;

    // Verificar permissão: User Management - read
    let _user_id = require_permission!(state, cookies, "User Management", "read");

    let page = 1u64;
    let per_page = 10u64;

    let (users_with_roles, total_pages) =
        UserService::find_users_with_roles_paginated(&state.conn, page, per_page)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let users: Vec<UserViewModel> = users_with_roles.into_iter().map(|u| u.into()).collect();

    let sidebar_user = get_sidebar_user(&cookies, &state).await;
    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    Ok(render_page(
        &headers,
        UserManagementFullTemplate {
            title: "User Management".into(),
            page_title: "User Management".into(),
            active_tab: "users".into(),
            users: Some(users.clone()),
            rbac: None,
            page,
            total_pages,
            sidebar_user,
            csrf_token: csrf_token.clone(),
        },
        UserManagementContentTemplate {
            active_tab: "users".into(),
            users: Some(users),
            rbac: None,
            page,
            total_pages,
            csrf_token,
        },
        "Bero Admin | User Management",
    ))
}

/// Handler para partial de usuários (HTMX)
pub async fn users_partial_handler(
    State(state): State<AppState>,
    Query(params): Query<UsersPageQuery>,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::service::user_service::UserService;

    // Verificar permissão: User Management - read
    let _user_id = require_permission!(state, cookies, "User Management", "read");

    let per_page = 10u64;
    let page = params.page.unwrap_or(1).max(1);

    let (users_with_roles, total_pages) =
        UserService::find_users_with_roles_paginated(&state.conn, page, per_page)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let users: Vec<UserViewModel> = users_with_roles.into_iter().map(|u| u.into()).collect();
    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    Ok(HtmlTemplate(UserManagementContentTemplate {
        active_tab: "users".into(),
        users: Some(users),
        rbac: None,
        page,
        total_pages,
        csrf_token,
    }))
}

/// Handler otimizado para paginação - retorna apenas a tabela
pub async fn users_table_handler(
    State(state): State<AppState>,
    Query(params): Query<UsersPageQuery>,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::service::rbac_service::check_permission;
    use crate::service::user_service::UserService;

    // Verificar permissão: User Management - read
    let user_id = require_permission!(state, cookies, "User Management", "read");

    let per_page = 10u64;
    let page = params.page.unwrap_or(1).max(1);

    let (users_with_roles, total_pages) =
        UserService::find_users_with_roles_paginated(&state.conn, page, per_page)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let users: Vec<UserViewModel> = users_with_roles.into_iter().map(|u| u.into()).collect();

    // Verificar permissões para botões de ação
    let mut redis_conn = state.redis.clone();
    let can_create = check_permission(
        &state.conn,
        &mut redis_conn,
        user_id,
        "User Management",
        "create",
    )
    .await;
    let can_edit = check_permission(
        &state.conn,
        &mut redis_conn,
        user_id,
        "User Management",
        "edit",
    )
    .await;
    let can_delete = check_permission(
        &state.conn,
        &mut redis_conn,
        user_id,
        "User Management",
        "delete",
    )
    .await;

    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    Ok(HtmlTemplate(UsersTableTemplate {
        users,
        page,
        total_pages,
        can_create,
        can_edit,
        can_delete,
        csrf_token,
    }))
}

/// Handler para formulário de criação de usuário
pub async fn admin_user_create_form_handler(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::middleware::csrf::get_csrf_token;
    use entity::roles;
    use sea_orm::{EntityTrait, QueryOrder};

    // Verificar permissão: User Management - create
    let _user_id = require_permission!(state, cookies, "User Management", "create");

    let roles = roles::Entity::find()
        .order_by_asc(roles::Column::Name)
        .all(&state.conn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(HtmlTemplate(UserFormPartialTemplate {
        roles: roles.into_iter().map(|r| r.into()).collect(),
        csrf_token: get_csrf_token(&cookies),
    }))
}

/// Handler para criação de usuário (POST)
pub async fn admin_user_store_handler(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    body: String,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use crate::dto::CreateUserParams;
    use crate::middleware::get_current_user_id;
    use crate::service::user_service::UserService;
    use crate::utils::security::sanitize_html;
    use validator::Validate;

    // Verificar permissão: User Management - create
    let _user_id = require_permission!(state, cookies, "User Management", "create");

    // Parse params manually to handle repeated role_ids
    let params_vec: Vec<(String, String)> = serde_urlencoded::from_str(&body).unwrap_or_default();

    let mut name = String::new();
    let mut user_name = String::new();
    let mut email = String::new();
    let mut password = String::new();
    let mut status = String::new();
    let mut role_ids_str: Vec<String> = Vec::new();

    for (k, v) in params_vec {
        match k.as_str() {
            "name" => name = v,
            "user" => user_name = v,
            "email" => email = v,
            "password" => password = v,
            "status" => status = v,
            "role_ids" => role_ids_str.push(v),
            _ => {}
        }
    }

    let name = sanitize_html(&name);
    let user_name = sanitize_html(&user_name);
    let email = sanitize_html(&email);

    let params = CreateUserParams {
        name: name.clone(),
        user: user_name.clone(),
        email: email.clone(),
        password: password.clone(),
        status: status.clone(),
        role_ids: role_ids_str.clone(),
    };

    if let Err(errors) = params.validate() {
        return Err(crate::error::AppError::ValidationErrors(errors));
    }

    let user = UserService::create_user_with_roles(&state.conn, params).await?;

    tracing::info!("User {} created", user.id);

    let current_user = get_current_user_id(&cookies, &state).await;

    // Enviar toast de sucesso
    // Enviar toast de sucesso
    crate::handlers::admin::toasts::toast(
        &state,
        current_user,
        crate::state::ToastLevel::Success,
        format!("Usuário '{}' criado com sucesso!", name),
    );

    crate::service::audit_service::AuditBuilder::new(&state.conn, "create", "user")
        .entity_id(user.id)
        .author(current_user)
        .ip(addr.ip().to_string())
        .log()
        .await;

    let partial_res = users_partial_handler(
        State(state),
        Query(UsersPageQuery { page: None }),
        cookies.clone(),
    )
    .await
    .map_err(|e| crate::error::AppError::InternalServerError(e.1))?
    .into_response();

    let partial_html = match axum::body::to_bytes(partial_res.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => String::from_utf8_lossy(&b).to_string(),
        Err(_) => "".to_string(),
    };

    use axum::response::Html;

    Ok(Html(partial_html))
}

/// Handler para formulário de edição de usuário
pub async fn admin_user_edit_form_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use entity::{roles, user_roles, users};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    // Verificar permissão: User Management - edit
    let _user_id = require_permission!(state, cookies, "User Management", "edit");

    let user = match users::Entity::find_by_id(id).one(&state.conn).await {
        Ok(Some(u)) => u,
        Ok(None) => return Err((StatusCode::NOT_FOUND, "User not found".to_string())),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    let roles = roles::Entity::find()
        .all(&state.conn)
        .await
        .unwrap_or_default();

    let current_role_ids: Vec<uuid::Uuid> = user_roles::Entity::find()
        .filter(user_roles::Column::UserId.eq(id))
        .all(&state.conn)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|ur| ur.role_id)
        .collect();

    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    Ok(HtmlTemplate(UserEditPartialTemplate {
        user,
        roles: roles.into_iter().map(|r| r.into()).collect(),
        current_role_ids,
        csrf_token,
    }))
}

/// Handler para atualização de usuário (PUT)
pub async fn admin_user_update_handler(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<uuid::Uuid>,
    body: String,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use crate::dto::UserUpdateParams;
    use crate::middleware::get_current_user_id;
    use crate::service::user_service::UserService;
    use crate::utils::security::sanitize_html;
    use validator::Validate;

    // Verificar permissão: User Management - edit
    let _user_id = require_permission!(state, cookies, "User Management", "edit");

    let params_vec: Vec<(String, String)> = serde_urlencoded::from_str(&body).unwrap_or_default();

    let mut name = String::new();
    let mut user_name = String::new();
    let mut email = String::new();
    let mut password = String::new();
    let mut status = String::new();
    let mut role_ids_str: Vec<String> = Vec::new();

    for (k, v) in params_vec {
        match k.as_str() {
            "name" => name = v,
            "user" => user_name = v,
            "email" => email = v,
            "password" => password = v,
            "status" => status = v,
            "role_ids" => role_ids_str.push(v),
            _ => {}
        }
    }

    let name = sanitize_html(&name);
    let user_name = sanitize_html(&user_name);
    let email = sanitize_html(&email);

    let params = UserUpdateParams {
        name: name.clone(),
        user: user_name.clone(),
        email: email.clone(),
        password: if password.is_empty() {
            None
        } else {
            Some(password.clone())
        },
        status: status.clone(),
        role_ids: role_ids_str.clone(),
    };

    if let Err(errors) = params.validate() {
        return Err(crate::error::AppError::ValidationErrors(errors));
    }

    let _updated_user = UserService::update_user_with_roles(&state.conn, id, params).await?;

    tracing::info!("User {} updated", id);

    // Invalidar cache do usuário atualizado
    crate::handlers::admin::users::invalidate_sidebar_cache(&state, id).await;

    let current_user = get_current_user_id(&cookies, &state).await;

    // Enviar toast de sucesso
    // Enviar toast de sucesso
    crate::handlers::admin::toasts::toast(
        &state,
        current_user,
        crate::state::ToastLevel::Success,
        "Usuário atualizado com sucesso!",
    );

    crate::service::audit_service::AuditBuilder::new(&state.conn, "update", "user")
        .entity_id(id)
        .author(current_user)
        .ip(addr.ip().to_string())
        .log()
        .await;

    let partial_res = users_partial_handler(
        State(state),
        Query(UsersPageQuery { page: None }),
        cookies.clone(),
    )
    .await
    .map_err(|e| crate::error::AppError::InternalServerError(e.1))?
    .into_response();

    let partial_html = match axum::body::to_bytes(partial_res.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => String::from_utf8_lossy(&b).to_string(),
        Err(_) => "".to_string(),
    };

    use axum::response::Html;

    Ok(Html(partial_html))
}

/// Handler para exclusão de usuário (DELETE)
/// Handler para exclusão de usuário (DELETE)
pub async fn admin_user_delete_handler(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use crate::middleware::get_current_user_id;
    use crate::service::user_service::UserService;

    // Verificar permissão: User Management - delete
    let _user_id = require_permission!(state, cookies, "User Management", "delete");

    UserService::delete_user_with_roles(&state.conn, id).await?;

    tracing::info!("User {} deleted", id);

    // Invalidar cache
    crate::handlers::admin::users::invalidate_sidebar_cache(&state, id).await;

    let current_user = get_current_user_id(&cookies, &state).await;

    // Enviar toast de sucesso
    // Enviar toast de sucesso
    crate::handlers::admin::toasts::toast(
        &state,
        current_user,
        crate::state::ToastLevel::Success,
        "Usuário excluído com sucesso!",
    );

    crate::service::audit_service::AuditBuilder::new(&state.conn, "delete", "user")
        .entity_id(id)
        .author(current_user)
        .ip(addr.ip().to_string())
        .log()
        .await;

    // Retorna resposta vazia - o HTMX usa hx-swap="delete" para remover a linha
    Ok(axum::http::StatusCode::OK)
}
