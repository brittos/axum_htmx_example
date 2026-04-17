//! Middleware de autenticação para rotas protegidas.
//!
//! Este módulo implementa verificação de sessão via cookies para
//! proteger rotas administrativas. A validação é feita contra Redis
//! (cache) e banco de dados (fallback).

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tower_cookies::Cookies;

use crate::state::AppState;

/// Nome do cookie de sessão
pub const SESSION_COOKIE_NAME: &str = "admin_session";

/// Middleware que requer autenticação com validação completa.
///
/// Verifica se existe um cookie de sessão válido e valida o token
/// contra Redis (cache) e banco de dados (fallback).
/// Se sessão inválida ou expirada, redireciona para login.
///
/// # Uso
/// ```rust,no_run
/// use axum::{Router, middleware, routing::get};
/// use axum_example_app::middleware::auth::require_auth;
/// use axum_example_app::state::AppState;
///
/// async fn dashboard() {}
///
/// # #[tokio::main]
/// # async fn main() {
/// // let state: AppState = ...;
/// // let app: Router = Router::new()
/// //     .route("/admin/dashboard", get(dashboard))
/// //     .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));
/// # }
/// ```
pub async fn require_auth(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    // Extrair cookies das extensions da request (adicionado pelo CookieManagerLayer)
    let cookies = request
        .extensions()
        .get::<Cookies>()
        .cloned()
        .ok_or_else(|| {
            tracing::error!("CookieManagerLayer not configured properly");
            redirect_to_login()
        })?;

    // Verificar se existe cookie de sessão
    let session_token = cookies.get(SESSION_COOKIE_NAME);

    match session_token {
        Some(cookie) => {
            let token_value = cookie.value();

            // Validar que o token não está vazio
            if token_value.is_empty() {
                tracing::warn!("Empty session token detected");
                return Err(redirect_to_login());
            }

            // Validar sessão contra Redis/banco de dados
            if validate_session_token(token_value, &state).await {
                tracing::debug!("Session validated successfully");
                Ok(next.run(request).await)
            } else {
                tracing::warn!("Invalid or expired session token");
                // Limpar cookie inválido - DEVE ter as mesmas propriedades do cookie original
                let expired_cookie = tower_cookies::Cookie::build((SESSION_COOKIE_NAME, ""))
                    .path("/")
                    .http_only(true)
                    .secure(state.config.cookie_secure)
                    .same_site(tower_cookies::cookie::SameSite::Lax)
                    .max_age(tower_cookies::cookie::time::Duration::seconds(0))
                    .build();
                cookies.remove(expired_cookie);
                Err(redirect_to_login())
            }
        }
        None => {
            tracing::debug!("No session cookie found, redirecting to login");
            Err(redirect_to_login())
        }
    }
}

/// Valida o token de sessão contra Redis (cache) e banco de dados.
async fn validate_session_token(token: &str, state: &AppState) -> bool {
    use entity::sessions;
    use redis::AsyncCommands;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let redis_key = format!("session:{}", token);
    let mut redis_conn = state.redis.clone();

    // 1. Tentar validar via Redis (cache)
    if let Ok(Some(_user_id)) = redis_conn.get::<_, Option<String>>(&redis_key).await {
        tracing::debug!("Session cache hit");
        return true;
    }

    // 2. Fallback: validar contra banco de dados
    let session_result = sessions::Entity::find()
        .filter(sessions::Column::Token.eq(token))
        .one(&state.conn)
        .await;

    match session_result {
        Ok(Some(session)) => {
            let now = chrono::Utc::now().fixed_offset();
            if session.expires_at < now {
                tracing::debug!("Session expired in database");
                // Limpar sessão expirada do banco
                let _ = sessions::Entity::delete_by_id(session.id)
                    .exec(&state.conn)
                    .await;
                // Limpar do Redis por precaução
                let _: Result<(), _> = redis_conn.del(&redis_key).await;
                return false;
            }

            // Sessão válida - cachear no Redis
            let ttl_seconds = (session.expires_at - now).num_seconds();
            if ttl_seconds > 0 {
                let _: Result<(), _> = redis_conn
                    .set_ex(&redis_key, session.user_id.to_string(), ttl_seconds as u64)
                    .await;
            }

            true
        }
        Ok(None) => {
            tracing::debug!("Session not found in database");
            false
        }
        Err(e) => {
            tracing::error!("Database error validating session: {}", e);
            // Em caso de erro de DB, negar acesso por segurança
            false
        }
    }
}

/// Redireciona para a página de login
fn redirect_to_login() -> Response {
    Redirect::to("/admin/login").into_response()
}

/// Middleware para verificar autenticação e retornar 401 (para APIs/HTMX)
///
/// Use este middleware quando você não quer redirecionar, mas sim
/// retornar um erro HTTP 401 Unauthorized.
pub async fn require_auth_api(
    cookies: Cookies,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let session_token = cookies.get(SESSION_COOKIE_NAME);

    match session_token {
        Some(cookie) if !cookie.value().is_empty() => Ok(next.run(request).await),
        _ => {
            tracing::debug!("API request without valid session");
            Err((
                StatusCode::UNAUTHORIZED,
                "Sessão expirada. Por favor, faça login novamente.",
            )
                .into_response())
        }
    }
}

/// Extrai o user_id do usuário logado a partir do cookie de sessão.
///
/// Busca a sessão no banco de dados pelo token do cookie.
/// Retorna None se a sessão não existir ou estiver expirada.
/// # Uso
/// ```rust,no_run
/// use axum_example_app::middleware::auth::get_current_user_id;
/// use axum_example_app::state::AppState;
/// use tower_cookies::Cookies;
///
/// # async fn example() {
/// let cookies: Cookies = todo!();
/// let state: AppState = todo!();
///
/// let current_user = get_current_user_id(&cookies, &state).await;
/// # }
/// ```
pub async fn get_current_user_id(
    cookies: &Cookies,
    state: &crate::state::AppState,
) -> Option<uuid::Uuid> {
    use entity::sessions;
    use redis::AsyncCommands;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    // Ler cookie de sessão
    let token = cookies.get(SESSION_COOKIE_NAME)?.value().to_string();

    if token.is_empty() {
        return None;
    }

    // 1. Tentar ler do Redis
    let redis_key = format!("session:{}", token);
    let mut redis_conn = state.redis.clone();

    if let Some(user_id) = redis_conn
        .get::<_, String>(&redis_key)
        .await
        .ok()
        .and_then(|id_str| uuid::Uuid::parse_str(&id_str).ok())
    {
        tracing::debug!("Session cache hit for token");
        return Some(user_id);
    }

    // 2. Buscar sessão no banco
    let session = sessions::Entity::find()
        .filter(sessions::Column::Token.eq(&token))
        .one(&state.conn)
        .await
        .ok()??;

    // Verificar se não expirou
    let now = chrono::Utc::now().fixed_offset();
    if session.expires_at < now {
        tracing::debug!("Session expired for token");
        // Remove do Redis por precaução
        let _ = redis_conn.del::<_, ()>(&redis_key).await;
        return None;
    }

    // 3. Salvar no Redis (TTL restante ou fixo)
    let ttl_seconds = (session.expires_at - now).num_seconds();
    if ttl_seconds > 0 {
        let _: Result<(), _> = redis_conn
            .set_ex(&redis_key, session.user_id.to_string(), ttl_seconds as u64)
            .await;
    }

    Some(session.user_id)
}

/// Retorna informações do usuário logado para exibição na sidebar.
///
/// Busca o nome, iniciais e role principal do usuário.
/// Retorna valores padrão se não conseguir buscar os dados.
pub async fn get_sidebar_user(
    cookies: &Cookies,
    state: &crate::state::AppState,
) -> crate::handlers::admin_templates::SidebarUserInfo {
    use entity::{roles, user_roles, users};
    use redis::AsyncCommands;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let default = crate::handlers::admin_templates::SidebarUserInfo::default();

    // Obter user_id da sessão
    let user_id = match get_current_user_id(cookies, state).await {
        Some(id) => id,
        None => return default,
    };

    // Tentar buscar do Redis
    let redis_key = format!("user:sidebar:{}", user_id);
    let mut redis_conn = state.redis.clone(); // ConnectionManager is clonable and cheap

    if let Some(cached_user) = redis_conn
        .get::<_, String>(&redis_key)
        .await
        .ok()
        .and_then(|json| {
            serde_json::from_str::<crate::handlers::admin_templates::SidebarUserInfo>(&json).ok()
        })
    {
        tracing::debug!("Sidebar user hit cache for {}", user_id);
        return cached_user;
    }

    // Se falhar, buscar do banco
    let conn = &state.conn;
    let user = match users::Entity::find_by_id(user_id).one(conn).await {
        Ok(Some(u)) => u,
        _ => return default,
    };

    // Gerar iniciais do nome
    let initials = user
        .name
        .split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .map(|c| c.to_ascii_uppercase())
        .collect::<String>();

    // Buscar role principal
    let role_name = match user_roles::Entity::find()
        .filter(user_roles::Column::UserId.eq(user_id))
        .one(conn)
        .await
    {
        Ok(Some(ur)) => match roles::Entity::find_by_id(ur.role_id).one(conn).await {
            Ok(Some(r)) => r.name,
            _ => "User".to_string(),
        },
        _ => "User".to_string(),
    };

    // Buscar permissões do usuário para exibição condicional no sidebar
    let permissions = crate::service::rbac_service::get_user_permissions(conn, user_id).await;

    let user_info = crate::handlers::admin_templates::SidebarUserInfo {
        name: user.name,
        initials: if initials.is_empty() {
            "U".to_string()
        } else {
            initials
        },
        role: role_name,
        permissions,
    };

    // Salvar no Redis (TTL 1 hora)
    if let Ok(json) = serde_json::to_string(&user_info) {
        let _: Result<(), _> = redis_conn.set_ex(&redis_key, json, 3600).await;
    }

    user_info
}

/// Invalida o cache da sidebar para um usuário específico.
pub async fn invalidate_sidebar_cache(state: &crate::state::AppState, user_id: uuid::Uuid) {
    use redis::AsyncCommands;
    let redis_key = format!("user:sidebar:{}", user_id);
    let mut redis_conn = state.redis.clone();

    if let Err(e) = redis_conn.del::<_, ()>(&redis_key).await {
        tracing::error!("Failed to invalidate sidebar cache for {}: {}", user_id, e);
    } else {
        tracing::debug!("Invalidated sidebar cache for {}", user_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_cookie_name() {
        assert_eq!(SESSION_COOKIE_NAME, "admin_session");
    }
}
