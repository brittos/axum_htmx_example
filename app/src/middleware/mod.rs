//! Middlewares da aplicação.
//!
//! Este módulo contém middlewares customizados para:
//! - Autenticação e autorização
//! - Proteção CSRF
//! - Logging (via tracing)
//! - Rate limiting customizado

pub mod auth;
pub mod csrf;
pub mod logging;
pub mod rate_limit_response;
pub mod rate_limits;

pub use auth::{
    SESSION_COOKIE_NAME, get_current_user_id, get_sidebar_user, invalidate_sidebar_cache,
    require_auth, require_auth_api,
};
pub use csrf::{CSRF_COOKIE_NAME, CSRF_HEADER_NAME, csrf_protection, get_csrf_token};
pub use rate_limit_response::rate_limit_response_middleware;
pub use rate_limits::{get_api_rate_limit_config, get_login_rate_limit_config};
