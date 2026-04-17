use crate::handlers::admin;
use crate::middleware::rate_limit_response::rate_limit_response_middleware;
use crate::middleware::rate_limits::get_api_rate_limit_config;
use crate::middleware::require_auth;
use crate::state::AppState;
use axum::{Router, middleware, routing::get};
use tower_governor::GovernorLayer;

pub fn protected_routes(state: AppState) -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/admin", get(admin::admin_dashboard))
        // Profile
        .route(
            "/admin/profile",
            get(admin::admin_profile_handler).post(admin::admin_profile_update_handler),
        )
        .route(
            "/admin/profile/password",
            axum::routing::post(admin::admin_profile_password_update_handler),
        )
        .route(
            "/admin/profile/sessions",
            get(admin::profile_sessions_handler),
        )
        .route(
            "/admin/profile/sessions/{id}/revoke",
            axum::routing::post(admin::profile_revoke_session_handler),
        )
        // User Management
        .route("/admin/users/management", get(admin::admin_users))
        .route("/admin/users/partial", get(admin::users_partial_handler))
        .route("/admin/users/table", get(admin::users_table_handler))
        .route(
            "/admin/users/create",
            get(admin::admin_user_create_form_handler),
        )
        .route(
            "/admin/users",
            axum::routing::post(admin::admin_user_store_handler),
        )
        .route(
            "/admin/users/{id}/edit",
            get(admin::admin_user_edit_form_handler),
        )
        .route(
            "/admin/users/{id}",
            axum::routing::put(admin::admin_user_update_handler)
                .delete(admin::admin_user_delete_handler),
        )
        // RBAC
        .route("/admin/rbac/partial", get(admin::rbac_partial_handler))
        .route(
            "/admin/rbac/toggle",
            axum::routing::patch(admin::rbac_toggle_handler),
        )
        // Posts
        .route(
            "/admin/posts",
            get(admin::admin_posts_handler).post(admin::admin_post_store_handler),
        )
        .route(
            "/admin/posts/create",
            get(admin::admin_post_create_form_handler),
        )
        .route(
            "/admin/posts/{id}/edit",
            get(admin::admin_post_edit_form_handler),
        )
        .route(
            "/admin/posts/{id}",
            axum::routing::put(admin::admin_post_update_handler)
                .delete(admin::admin_post_delete_handler),
        )
        // Settings
        .route("/admin/settings", get(admin::admin_settings))
        // Audit Logs
        .route("/admin/audit-logs", get(admin::audit_logs_handler))
        .route(
            "/admin/audit-logs/partial",
            get(admin::audit_logs_partial_handler),
        )
        .route(
            "/admin/audit-logs/export-csv",
            get(admin::audit_logs_export_csv_handler),
        )
        // Notifications
        .route(
            "/admin/notifications/partial",
            get(admin::notifications_partial_handler),
        )
        .route(
            "/admin/notifications/{id}/read",
            axum::routing::post(admin::mark_notification_read_handler),
        )
        .route(
            "/admin/notifications/read-all",
            axum::routing::post(admin::mark_all_notifications_read_handler),
        )
        .route(
            "/admin/notifications/status",
            get(admin::check_status_handler),
        )
        .route(
            "/admin/notifications/close",
            get(admin::close_notifications_handler),
        )
        // Toast SSE Stream
        .route(
            "/admin/toasts/stream",
            get(admin::toasts::toast_stream_handler),
        )
        .route(
            "/admin/toasts/dismiss",
            axum::routing::delete(admin::toasts::dismiss_toast_handler),
        )
        // Middleware de autenticação e Rate Limit
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .layer(GovernorLayer::new(get_api_rate_limit_config()))
        .layer(middleware::from_fn(rate_limit_response_middleware))
}
