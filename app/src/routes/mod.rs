use crate::handlers::error_handler;
use crate::middleware::csrf_protection;
use crate::state::AppState;
use axum::{Router, extract::DefaultBodyLimit, http::HeaderValue, middleware};
use tower_cookies::CookieManagerLayer;
use tower_http::{
    compression::CompressionLayer, set_header::SetResponseHeaderLayer, trace::TraceLayer,
};

pub mod admin;
pub mod public;

pub fn configure_routes(state: AppState) -> Router {
    use axum::http::header::{
        CONTENT_SECURITY_POLICY, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
        X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
    };

    let login_routes = public::login_routes();
    let public_routes = public::public_routes();
    let protected_routes = admin::protected_routes(state.clone());

    Router::<AppState>::new()
        .merge(login_routes)
        .merge(public_routes)
        .merge(protected_routes)
        .fallback(error_handler::handler_404)
        // CSRF protection
        .layer(middleware::from_fn(csrf_protection))
        // Cookie management
        .layer(CookieManagerLayer::new())
        // Body size limit (1MB)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        // Response compression
        .layer(CompressionLayer::new())
        // Security headers
        .layer(SetResponseHeaderLayer::overriding(
            X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; \
                 script-src 'self' 'unsafe-inline' https://unpkg.com; \
                 style-src 'self' https://fonts.googleapis.com 'unsafe-inline'; \
                 font-src 'self' https://fonts.gstatic.com; \
                 img-src 'self' data: https://placehold.co https://images.unsplash.com; \
                 connect-src 'self'",
            ),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static(
                "camera=(), microphone=(), geolocation=(), interest-cohort=()",
            ),
        ))
        // Observability
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
