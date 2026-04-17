//! Handlers do dashboard administrativo.

use crate::handlers::admin_templates::{DashboardContentTemplate, DashboardFullTemplate};
use crate::handlers::response::render_page;
use crate::middleware::get_sidebar_user;
use crate::require_permission;
use crate::state::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use tower_cookies::Cookies;

/// Handler para dashboard principal
pub async fn admin_dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use entity::users;
    use sea_orm::{EntityTrait, PaginatorTrait};

    // Verificar permissão: Dashboard - read
    let _user_id = require_permission!(state, cookies, "Dashboard", "read");

    let active_users = users::Entity::find().count(&state.conn).await.unwrap_or(0) as u32;

    let (total_posts, server_load, system_health) = (150, 34, "98%".to_string());

    let sidebar_user = get_sidebar_user(&cookies, &state).await;
    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    Ok(render_page(
        &headers,
        DashboardFullTemplate {
            title: "Dashboard".into(),
            page_title: "Dashboard".into(),
            active_tab: "dashboard".into(),
            active_users,
            total_posts,
            server_load,
            system_health: system_health.clone(),
            sidebar_user,
            csrf_token,
        },
        DashboardContentTemplate {
            active_users,
            total_posts,
            server_load,
            system_health,
        },
        "Bero Admin | Dashboard",
    ))
}
