//! Handlers de autenticação (login/logout).

use crate::handlers::admin_templates::LoginTemplate;
use crate::handlers::response::HtmlTemplate;
use crate::middleware::SESSION_COOKIE_NAME;
use crate::state::AppState;
use axum::extract::ConnectInfo;
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::net::SocketAddr;
use tower_cookies::{Cookie, Cookies};

/// Parâmetros da query do login
#[derive(Deserialize)]
pub struct LoginQueryParams {
    pub reset_success: Option<bool>,
    pub error: Option<String>,
}

/// Parâmetros do formulário de login
#[derive(Deserialize)]
pub struct LoginParams {
    pub username: String,
    pub password: String,
}

/// Retorna resposta de erro de login com headers HTMX (Opção B)
fn login_error_response(msg: &str) -> Response {
    let html = format!(r#"<div class="alert alert--error">{}</div>"#, msg);
    (
        [("HX-Retarget", "#login-error"), ("HX-Reswap", "innerHTML")],
        Html(html),
    )
        .into_response()
}

/// Handler GET para exibir página de login
pub async fn admin_login(
    axum::extract::Query(query): axum::extract::Query<LoginQueryParams>,
) -> impl IntoResponse {
    let success_message = if query.reset_success.unwrap_or(false) {
        Some("Senha alterada com sucesso! Faça login com sua nova senha.".to_string())
    } else {
        None
    };

    HtmlTemplate(LoginTemplate {
        title: "Login".into(),
        page_title: "Login".into(),
        active_tab: "".into(),
        success_message,
        error_message: query.error,
    })
}

/// Handler POST para processar login
pub async fn admin_login_post(
    State(state): State<AppState>,
    cookies: Cookies,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Form(params): Form<LoginParams>,
) -> impl IntoResponse {
    use crate::service::login_service::{
        LoginAttemptResult, can_attempt_login, clear_attempts, record_failed_attempt,
    };
    use crate::utils::password::verify_password;
    use entity::users;
    use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter};

    let ip = addr.ip().to_string();
    let user_agent = headers
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Verificar se está bloqueado por tentativas excessivas
    match can_attempt_login(
        &state.conn,
        &params.username,
        &ip,
        state.config.max_login_attempts,
    )
    .await
    {
        Ok(LoginAttemptResult::Blocked { minutes_remaining }) => {
            tracing::warn!(
                "Login blocked for user {} from IP {} ({} minutes remaining)",
                params.username,
                ip,
                minutes_remaining
            );
            return login_error_response(&format!(
                "Muitas tentativas. Tente novamente em {} minutos.",
                minutes_remaining
            ));
        }
        Err(e) => {
            tracing::error!("Error checking login attempts: {}", e);
            // Em caso de erro, permitir tentativa (fail-open)
        }
        Ok(LoginAttemptResult::Allowed) => {}
    }

    // Buscar usuário por username ou email
    let user_result = users::Entity::find()
        .filter(
            Condition::any()
                .add(users::Column::Username.eq(&params.username))
                .add(users::Column::Email.eq(&params.username)),
        )
        .one(&state.conn)
        .await;

    match user_result {
        Ok(Some(user)) => {
            // Verificar senha
            if verify_password(&params.password, &user.password) {
                // Verificar se usuário está ativo
                if user.status != "Active" {
                    tracing::warn!("Login attempt for inactive user: {}", user.username);
                    return login_error_response("Conta inativa. Contate o administrador.");
                }

                // Limpar tentativas após sucesso
                let _ = clear_attempts(&state.conn, &params.username, &ip).await;

                // Criar token de sessão (UUID v7 para ordenação temporal)
                let session_id = uuid::Uuid::now_v7();
                let session_token = uuid::Uuid::now_v7().to_string();
                let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

                // Salvar sessão no banco
                {
                    use entity::sessions;
                    use sea_orm::{ActiveModelTrait, Set};

                    let session = sessions::ActiveModel {
                        id: Set(session_id),
                        user_id: Set(user.id),
                        token: Set(session_token.clone()),
                        expires_at: Set(expires_at.fixed_offset()),
                        created_at: Set(chrono::Utc::now().fixed_offset()),

                        ip_address: Set(Some(addr.ip().to_string())),
                        user_agent: Set(user_agent),
                    };

                    if let Err(e) = session.insert(&state.conn).await {
                        tracing::error!("Failed to create session: {}", e);
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Erro ao criar sessão")
                            .into_response();
                    }
                }

                // Criar cookie seguro
                let cookie = Cookie::build((SESSION_COOKIE_NAME, session_token.clone()))
                    .http_only(true)
                    .secure(state.config.cookie_secure)
                    .same_site(tower_cookies::cookie::SameSite::Lax)
                    .path("/")
                    .max_age(tower_cookies::cookie::time::Duration::hours(24))
                    .build();

                cookies.add(cookie);

                tracing::info!("User {} logged in successfully", user.username);

                // Verificar sessões simultâneas
                if let Ok(active_count) =
                    crate::service::session_service::count_active_sessions(&state.conn, user.id)
                        .await
                    && active_count > 1
                {
                    tracing::info!(
                        "User {} has {} concurrent sessions",
                        user.username,
                        active_count
                    );
                    // Log de auditoria para sessões simultâneas
                    crate::service::audit_service::log_action(
                        &state.conn,
                        "concurrent_session",
                        "session",
                        Some(session_id),
                        Some(user.id),
                        Some(addr.ip().to_string()),
                        Some(format!("Total active sessions: {}", active_count)),
                    )
                    .await;
                }

                // Log de auditoria do login
                crate::service::audit_service::log_action(
                    &state.conn,
                    "login",
                    "user",
                    Some(user.id),
                    Some(user.id),
                    Some(addr.ip().to_string()),
                    None,
                )
                .await;

                // Redirecionar via HTMX
                ([("HX-Redirect", "/admin")], "").into_response()
            } else {
                // Registrar tentativa falha
                let _ = record_failed_attempt(
                    &state.conn,
                    &params.username,
                    &ip,
                    state.config.max_login_attempts,
                    state.config.login_lockout_minutes,
                )
                .await;

                tracing::warn!("Invalid password for user: {}", params.username);
                login_error_response("Usuário ou senha inválidos")
            }
        }
        Ok(None) => {
            // Registrar tentativa falha mesmo para usuário inexistente
            let _ = record_failed_attempt(
                &state.conn,
                &params.username,
                &ip,
                state.config.max_login_attempts,
                state.config.login_lockout_minutes,
            )
            .await;

            tracing::warn!("Login attempt for non-existent user: {}", params.username);
            login_error_response("Usuário ou senha inválidos")
        }
        Err(e) => {
            tracing::error!("Database error during login: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Erro interno").into_response()
        }
    }
}

/// Handler para logout
pub async fn admin_logout(
    State(state): State<AppState>,
    cookies: Cookies,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    use entity::sessions;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    // Obter token da sessão atual
    let session_token = cookies
        .get(SESSION_COOKIE_NAME)
        .map(|c| c.value().to_string());

    let mut user_id: Option<uuid::Uuid> = None;

    // Invalidar sessão no banco de dados
    if let Some(token) = session_token {
        // Buscar sessão para obter user_id (para auditoria)
        if let Ok(Some(session)) = sessions::Entity::find()
            .filter(sessions::Column::Token.eq(&token))
            .one(&state.conn)
            .await
        {
            user_id = Some(session.user_id);

            // Deletar sessão do banco
            if let Err(e) = sessions::Entity::delete_by_id(session.id)
                .exec(&state.conn)
                .await
            {
                tracing::warn!("Failed to delete session from database: {}", e);
            } else {
                tracing::debug!("Session deleted from database");
            }
        }
    }

    // Remover cookie de sessão - DEVE ter as mesmas propriedades do cookie original
    let cookie = Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(tower_cookies::cookie::SameSite::Lax)
        .max_age(tower_cookies::cookie::time::Duration::seconds(0))
        .build();

    cookies.remove(cookie);

    // Log de auditoria
    if let Some(uid) = user_id {
        crate::service::audit_service::log_action(
            &state.conn,
            "logout",
            "user",
            Some(uid),
            Some(uid),
            Some(addr.ip().to_string()),
            None,
        )
        .await;
    }

    tracing::info!("User logged out");

    Redirect::to("/admin/login")
}
