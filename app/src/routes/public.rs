use crate::handlers::{admin, auth_recovery};
use crate::middleware::rate_limit_response::rate_limit_response_middleware;
use crate::middleware::rate_limits::get_login_rate_limit_config;
use crate::state::AppState;
use axum::{Router, middleware, routing::get};
use tower_governor::GovernorLayer;

// Debug mode: serve from filesystem (hot reload)
#[cfg(debug_assertions)]
use tower_http::services::{ServeDir, ServeFile};

// Release mode: serve from embedded assets
#[cfg(not(debug_assertions))]
use crate::embedded_assets::StaticAssets;
#[cfg(not(debug_assertions))]
use axum_embed::ServeEmbed;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/admin/logout", get(admin::admin_logout))
        .merge(static_routes())
}

/// Static file routes - uses filesystem in debug, embedded in release
#[cfg(debug_assertions)]
fn static_routes() -> Router<AppState> {
    Router::new()
        .route_service("/favicon.ico", ServeFile::new("app/static/favicon.ico"))
        .nest_service("/static", ServeDir::new("app/static"))
}

#[cfg(not(debug_assertions))]
fn static_routes() -> Router<AppState> {
    use axum::{
        body::Body,
        http::{StatusCode, header},
        response::{IntoResponse, Response},
    };

    // Handler for favicon.ico - serves from embedded assets
    async fn favicon_handler() -> Response {
        match StaticAssets::get("favicon.ico") {
            Some(content) => {
                let body = Body::from(content.data.to_vec());
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "image/x-icon")
                    .body(body)
                    .unwrap()
            }
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }

    let serve_assets = ServeEmbed::<StaticAssets>::new();
    Router::new()
        .route("/favicon.ico", get(favicon_handler))
        .nest_service("/static", serve_assets)
}

pub fn login_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/login",
            get(admin::admin_login).post(admin::admin_login_post),
        )
        // Password Recovery
        .route(
            "/auth/forgot-password",
            get(auth_recovery::forgot_password_page).post(auth_recovery::forgot_password_submit),
        )
        .route(
            "/auth/reset-password",
            get(auth_recovery::reset_password_page).post(auth_recovery::reset_password_submit),
        )
        .layer(GovernorLayer::new(get_login_rate_limit_config()))
        .layer(middleware::from_fn(rate_limit_response_middleware))
}
