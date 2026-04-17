//! Proteção CSRF usando o padrão Double Submit Cookie.
//!
//! Para HTMX, o token é lido do cookie e enviado via header `X-CSRF-Token`.
//!
//! Configuração no template base:
//! ```html
//! <script>
//!     document.body.addEventListener('htmx:configRequest', (event) => {
//!         const csrfToken = document.cookie.match(/csrf_token=([^;]+)/)?.[1];
//!         if (csrfToken) {
//!             event.detail.headers['X-CSRF-Token'] = csrfToken;
//!         }
//!     });
//! </script>
//! ```

use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;

/// Nome do cookie CSRF
pub const CSRF_COOKIE_NAME: &str = "csrf_token";

/// Nome do header CSRF esperado em requests HTMX
pub const CSRF_HEADER_NAME: &str = "x-csrf-token";

/// Verifica se cookies devem ser marcados como secure (requer HTTPS)
/// Lê da variável de ambiente COOKIE_SECURE (consistente com AppConfig)
fn is_cookie_secure() -> bool {
    std::env::var("COOKIE_SECURE")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Middleware que valida token CSRF em requests que modificam estado.
///
/// Métodos verificados: POST, PUT, PATCH, DELETE
/// Métodos permitidos sem token: GET, HEAD, OPTIONS
///
/// O token pode ser enviado via:
/// - Header `X-CSRF-Token`
/// - Query parameter `csrf_token`
pub async fn csrf_protection(
    cookies: Cookies,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let method = request.method().clone();
    let uri = request.uri().clone();

    // Métodos seguros não precisam de verificação
    if matches!(method, Method::GET | Method::HEAD | Method::OPTIONS) {
        // Garantir que existe um token CSRF no cookie
        ensure_csrf_cookie(&cookies);
        return Ok(next.run(request).await);
    }

    // Exceção para rota de login (primeira requisição, sem cookie ainda)
    if uri.path() == "/admin/login" {
        ensure_csrf_cookie(&cookies);
        return Ok(next.run(request).await);
    }

    // Exceção para toast dismiss (operação UI apenas, sem side-effects de segurança)
    if uri.path() == "/admin/toasts/dismiss" {
        return Ok(next.run(request).await);
    }

    // Para métodos que modificam estado, verificar token
    let cookie_token = cookies.get(CSRF_COOKIE_NAME).map(|c| c.value().to_string());

    let header_token = request
        .headers()
        .get(CSRF_HEADER_NAME)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Fallback: tentar query parameter (para formulários tradicionais)
    let query_token = request.uri().query().and_then(|q| {
        q.split('&')
            .find(|p| p.starts_with("csrf_token="))
            .map(|p| p.trim_start_matches("csrf_token=").to_string())
    });

    let submitted_token = header_token.or(query_token);

    match (cookie_token, submitted_token) {
        (Some(cookie), Some(submitted)) if constant_time_compare(&cookie, &submitted) => {
            // Token válido, continuar
            Ok(next.run(request).await)
        }
        (None, _) => {
            tracing::warn!("CSRF validation failed: no cookie token");
            Err((StatusCode::FORBIDDEN, "CSRF token inválido").into_response())
        }
        (_, None) => {
            tracing::warn!(
                "CSRF validation failed: no submitted token for {}",
                uri.path()
            );
            Err((StatusCode::FORBIDDEN, "CSRF token ausente").into_response())
        }
        _ => {
            tracing::warn!("CSRF validation failed: token mismatch");
            Err((StatusCode::FORBIDDEN, "CSRF token inválido").into_response())
        }
    }
}

/// Garante que existe um cookie CSRF, gerando um se necessário
fn ensure_csrf_cookie(cookies: &Cookies) {
    if cookies.get(CSRF_COOKIE_NAME).is_none() {
        let token = Uuid::now_v7().to_string();
        let cookie = Cookie::build((CSRF_COOKIE_NAME, token))
            .http_only(false) // Precisa ser acessível via JS para HTMX
            .secure(is_cookie_secure())
            .same_site(tower_cookies::cookie::SameSite::Lax)
            .path("/")
            .build();
        cookies.add(cookie);
    }
}

/// Comparação constant-time para evitar timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// Extrator para obter o token CSRF atual (para injetar em templates)
pub fn get_csrf_token(cookies: &Cookies) -> String {
    cookies
        .get(CSRF_COOKIE_NAME)
        .map(|c| c.value().to_string())
        .unwrap_or_else(|| {
            let token = Uuid::now_v7().to_string();
            let cookie = Cookie::build((CSRF_COOKIE_NAME, token.clone()))
                .http_only(false)
                .secure(is_cookie_secure())
                .same_site(tower_cookies::cookie::SameSite::Lax)
                .path("/")
                .build();
            cookies.add(cookie);
            token
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("abc", "abc"));
        assert!(!constant_time_compare("abc", "abd"));
        assert!(!constant_time_compare("abc", "ab"));
    }
}
