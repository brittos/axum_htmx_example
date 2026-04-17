//! Handlers para gerenciamento de perfil do usuário.

use crate::handlers::admin_templates::{ProfileContentTemplate, ProfileFullTemplate};
use crate::handlers::response::render_page;
use crate::middleware::{
    get_csrf_token, get_current_user_id, get_sidebar_user, invalidate_sidebar_cache,
};
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set};
use serde::Deserialize;
use tower_cookies::Cookies;

#[derive(Deserialize)]
pub struct UpdateProfileParams {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct UpdatePasswordParams {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

#[derive(Deserialize)]
pub struct ProfileQueryParams {
    pub success: Option<String>,
    pub error: Option<String>,
}

/// GET /admin/profile
pub async fn admin_profile_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    cookies: Cookies,
    Query(query): Query<ProfileQueryParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use entity::users;

    // 1. Obter ID do usuário logado
    let user_id = get_current_user_id(&cookies, &state)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "Sessão inválida".to_string()))?;

    // 2. Buscar dados do usuário
    let user = users::Entity::find_by_id(user_id)
        .one(&state.conn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Usuário não encontrado".to_string()))?;

    // 3. Obter token CSRF
    let csrf_token = get_csrf_token(&cookies);

    let success_message = if query.success.is_some() {
        Some("Changes saved successfully.".to_string())
    } else {
        None
    };

    let error_message = query.error.clone();

    let sidebar_user = get_sidebar_user(&cookies, &state).await;

    Ok(render_page(
        &headers,
        ProfileFullTemplate {
            title: "My Profile".into(),
            page_title: "My Profile".into(),
            active_tab: "profile".into(),
            user: user.clone(),
            success_message: success_message.clone(),
            error_message: error_message.clone(),
            csrf_token: csrf_token.clone(),
            sidebar_user,
        },
        ProfileContentTemplate {
            user,
            success_message,
            error_message,
            csrf_token,
        },
        "Bero Admin | My Profile",
    ))
}

/// POST /admin/profile (Update Info)
pub async fn admin_profile_update_handler(
    State(state): State<AppState>,
    cookies: Cookies,
    headers: HeaderMap,
    Form(params): Form<UpdateProfileParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use entity::users;

    let user_id = get_current_user_id(&cookies, &state)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "Sessão inválida".to_string()))?;

    let user = users::Entity::find_by_id(user_id)
        .one(&state.conn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Usuário não encontrado".to_string()))?;

    let mut active_user = user.into_active_model();

    active_user.name = Set(params.name);
    active_user.email = Set(params.email);

    active_user
        .update(&state.conn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Invalidar cache
    invalidate_sidebar_cache(&state, user_id).await;

    // Enviar toast de sucesso
    crate::handlers::admin::toasts::send_toast(
        &state,
        user_id,
        crate::state::ToastLevel::Success,
        "Perfil atualizado com sucesso!",
    );

    // Se for HTMX, retorna o partial com mensagem de sucesso
    if headers.contains_key("HX-Request") {
        let csrf_token = crate::middleware::csrf::get_csrf_token(&cookies);
        let updated_user = users::Entity::find_by_id(user_id)
            .one(&state.conn)
            .await
            .unwrap()
            .unwrap();

        let partial = ProfileContentTemplate {
            user: updated_user,
            success_message: Some("Perfil atualizado com sucesso!".to_string()),
            error_message: None,
            csrf_token,
        };

        Ok(Html(partial.render().unwrap_or_default()).into_response())
    } else {
        Ok(Redirect::to("/admin/profile?success=true").into_response())
    }
}

/// POST /admin/profile/password (Update Password)
pub async fn admin_profile_password_update_handler(
    State(state): State<AppState>,
    cookies: Cookies,
    headers: HeaderMap,
    Form(params): Form<UpdatePasswordParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::utils::password::{hash_password, verify_password};
    use entity::users;

    let user_id = get_current_user_id(&cookies, &state)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "Sessão inválida".to_string()))?;

    let user = users::Entity::find_by_id(user_id)
        .one(&state.conn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Usuário não encontrado".to_string()))?;

    let csrf_token = crate::middleware::csrf::get_csrf_token(&cookies);

    let render_error = |user: entity::users::Model, msg: &str| {
        let p = ProfileContentTemplate {
            user,
            success_message: None,
            error_message: Some(msg.to_string()),
            csrf_token: csrf_token.clone(),
        };
        Html(p.render().unwrap_or_default())
    };

    // 1. Validações básicas
    if params.new_password != params.confirm_password {
        if headers.contains_key("HX-Request") {
            return Ok(render_error(user, "As senhas não coincidem").into_response());
        } else {
            return Ok(
                Redirect::to("/admin/profile?error=As+senhas+n%C3%A3o+coincidem").into_response(),
            );
        }
    }

    if params.new_password.len() < 8 {
        if headers.contains_key("HX-Request") {
            return Ok(
                render_error(user, "A senha deve ter pelo menos 8 caracteres").into_response(),
            );
        } else {
            return Ok(Redirect::to("/admin/profile?error=Senha+muito+curta").into_response());
        }
    }

    // 3. Verificar senha atual
    if !verify_password(&params.current_password, &user.password) {
        if headers.contains_key("HX-Request") {
            return Ok(render_error(user, "Senha atual incorreta").into_response());
        } else {
            return Ok(Redirect::to("/admin/profile?error=Senha+incorreta").into_response());
        }
    }

    // 4. Hash e update
    let new_hash = hash_password(&params.new_password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut active_user = user.into_active_model();
    active_user.password = Set(new_hash);

    active_user
        .update(&state.conn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Enviar toast de sucesso
    crate::handlers::admin::toasts::send_toast(
        &state,
        user_id,
        crate::state::ToastLevel::Success,
        "Senha alterada com sucesso!",
    );

    if headers.contains_key("HX-Request") {
        let updated_user = users::Entity::find_by_id(user_id)
            .one(&state.conn)
            .await
            .unwrap()
            .unwrap();

        let partial = ProfileContentTemplate {
            user: updated_user,
            success_message: Some("Senha alterada com sucesso!".to_string()),
            error_message: None,
            csrf_token,
        };

        Ok(Html(partial.render().unwrap_or_default()).into_response())
    } else {
        Ok(Redirect::to("/admin/profile?success=true").into_response())
    }
}

// --- Active Sessions ---

#[derive(serde::Serialize)]
pub struct SessionViewModel {
    pub id: String,
    pub ip_address: String,
    pub user_agent: String,
    pub created_at: String,
    pub last_accessed: String,
    pub is_current: bool,
    pub expires_in: String,
}

#[derive(Template)]
#[template(path = "admin/profile_sessions_partial.html")]
pub struct ProfileSessionsTemplate {
    pub sessions: Vec<SessionViewModel>,
    pub csrf_token: String,
}

/// GET /admin/profile/sessions
pub async fn profile_sessions_handler(
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::middleware::auth::SESSION_COOKIE_NAME;
    use crate::service::session_service;

    let user_id = get_current_user_id(&cookies, &state)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "Sessão inválida".to_string()))?;

    let current_token = cookies
        .get(SESSION_COOKIE_NAME)
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    let sessions = session_service::list_active_sessions(&state.conn, user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let view_models: Vec<SessionViewModel> = sessions
        .into_iter()
        .map(|s| {
            let is_current = s.token == current_token;
            let created: chrono::DateTime<chrono::Utc> = s.created_at.into();
            let expires: chrono::DateTime<chrono::Utc> = s.expires_at.into();
            let now = chrono::Utc::now();

            // Calcular tempo restante
            let duration = expires.signed_duration_since(now);
            let expires_in = if duration.num_minutes() < 60 {
                format!("{} min", duration.num_minutes())
            } else {
                format!("{} h", duration.num_hours())
            };

            let duration_since_creation = now.signed_duration_since(created);
            let last_accessed = if duration_since_creation.num_minutes() < 1 {
                "Agora mesmo".to_string()
            } else if duration_since_creation.num_minutes() < 60 {
                format!("{}m atrás", duration_since_creation.num_minutes())
            } else if duration_since_creation.num_hours() < 24 {
                format!("{}h atrás", duration_since_creation.num_hours())
            } else {
                format!("{}d atrás", duration_since_creation.num_days())
            };

            SessionViewModel {
                id: s.id.to_string(),
                ip_address: s.ip_address.unwrap_or_else(|| "Desconhecido".to_string()),
                user_agent: s.user_agent.unwrap_or_else(|| "Desconhecido".to_string()),
                created_at: created.format("%d/%m/%Y %H:%M").to_string(),
                last_accessed,
                is_current,
                expires_in,
            }
        })
        .collect();

    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    let template = ProfileSessionsTemplate {
        sessions: view_models,
        csrf_token,
    };

    Ok(Html(template.render().unwrap_or_default()))
}

/// POST /admin/profile/sessions/{id}/revoke
pub async fn profile_revoke_session_handler(
    State(state): State<AppState>,
    cookies: Cookies,
    Path(session_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::middleware::auth::SESSION_COOKIE_NAME;
    use entity::sessions;

    let user_id = get_current_user_id(&cookies, &state)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "Sessão inválida".to_string()))?;

    let session_uuid = uuid::Uuid::parse_str(&session_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "ID inválido".to_string()))?;

    // Verificar se a sessão pertence ao usuário
    let session = sessions::Entity::find_by_id(session_uuid)
        .filter(sessions::Column::UserId.eq(user_id))
        .one(&state.conn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Sessão não encontrada".to_string()))?;

    // Verificar se é a sessão atual (opcional: impedir auto-revogação via botão, ou permitir logout)
    let current_token = cookies
        .get(SESSION_COOKIE_NAME)
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    if session.token == current_token {
        return Err((
            StatusCode::BAD_REQUEST,
            "Não é possível revogar a sessão atual por aqui. Use o botão de Logout.".to_string(),
        ));
    }

    // Deletar do banco
    let res = sessions::Entity::delete_by_id(session_uuid)
        .exec(&state.conn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if res.rows_affected > 0 {
        // Remover do Redis
        let redis_key = format!("session:{}", session.token);
        let mut redis = state.redis.clone();
        let _ = redis::AsyncCommands::del::<_, ()>(&mut redis, &redis_key).await;

        // Toast success
        crate::handlers::admin::toasts::send_toast(
            &state,
            user_id,
            crate::state::ToastLevel::Success,
            "Sessão revogada com sucesso",
        );
    }

    // Retornar a lista atualizada
    profile_sessions_handler(State(state), cookies).await
}
