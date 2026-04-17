//! Handlers de configurações administrativas.

use crate::handlers::admin_templates::{SettingsContentTemplate, SettingsFullTemplate};
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

/// Handler para página de configurações
pub async fn admin_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Verificar permissão: Settings - read
    let _user_id = require_permission!(state, cookies, "Settings", "read");

    let sidebar_user = get_sidebar_user(&cookies, &state).await;

    Ok(render_page(
        &headers,
        SettingsFullTemplate {
            title: "Settings".into(),
            page_title: "Settings".into(),
            active_tab: "settings".into(),
            sidebar_user,
        },
        SettingsContentTemplate {},
        "Bero Admin | Settings",
    ))
}
