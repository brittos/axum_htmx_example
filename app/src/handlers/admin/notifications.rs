//! Handlers para gerenciamento de notificações in-app.

use crate::middleware::get_current_user_id;
use crate::service::notification_service;
use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use serde::Serialize;
use tower_cookies::Cookies;

/// ViewModel para notificação
#[derive(Clone, Serialize)]
pub struct NotificationViewModel {
    pub id: String,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub is_read: bool,
    pub action_url: Option<String>,
    pub created_at: String,
    pub time_ago: String,
}

impl From<entity::notifications::Model> for NotificationViewModel {
    fn from(n: entity::notifications::Model) -> Self {
        use chrono::{DateTime, Utc};

        let created: DateTime<Utc> = n.created_at.into();
        let now = Utc::now();
        let diff = now.signed_duration_since(created);

        let time_ago = if diff.num_minutes() < 1 {
            "agora".to_string()
        } else if diff.num_minutes() < 60 {
            format!("{}m atrás", diff.num_minutes())
        } else if diff.num_hours() < 24 {
            format!("{}h atrás", diff.num_hours())
        } else {
            format!("{}d atrás", diff.num_days())
        };

        Self {
            id: n.id.to_string(),
            title: n.title,
            message: n.message,
            notification_type: n.notification_type,
            is_read: n.is_read,
            action_url: n.action_url,
            created_at: n.created_at.format("%d/%m/%Y %H:%M").to_string(),
            time_ago,
        }
    }
}

/// Template para dropdown de notificações (partial HTMX)
#[derive(Template)]
#[template(path = "admin/notifications_partial.html")]
pub struct NotificationsPartialTemplate {
    pub notifications: Vec<NotificationViewModel>,
    pub unread_count: u64,
    pub csrf_token: String,
}

/// GET /admin/notifications/partial - Dropdown HTMX
pub async fn notifications_partial_handler(
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id = get_current_user_id(&cookies, &state)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "Não autenticado".to_string()))?;

    let notifications = notification_service::list_unread(&state.conn, user_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .take(5)
        .map(NotificationViewModel::from)
        .collect();

    let unread_count = notification_service::count_unread(&state.conn, user_id)
        .await
        .unwrap_or(0);

    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    let template = NotificationsPartialTemplate {
        notifications,
        unread_count,
        csrf_token,
    };

    Ok(Html(template.render().unwrap()))
}

/// POST /admin/notifications/{id}/read - Marca notificação como lida
pub async fn mark_notification_read_handler(
    State(state): State<AppState>,
    cookies: Cookies,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id = get_current_user_id(&cookies, &state)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "Não autenticado".to_string()))?;

    let notification_id = uuid::Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "ID inválido".to_string()))?;

    notification_service::mark_as_read(&state.conn, notification_id, user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Recarrega dropdown e contagem
    let notifications = notification_service::list_unread(&state.conn, user_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .take(5)
        .map(NotificationViewModel::from)
        .collect();

    let unread_count = notification_service::count_unread(&state.conn, user_id)
        .await
        .unwrap_or(0);

    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    let dropdown_template = NotificationsPartialTemplate {
        notifications,
        unread_count,
        csrf_token,
    };

    let bell_template = NotificationBellTemplate {
        has_unread: unread_count > 0,
    };

    // Concatena as duas templates (Dropdown atualiza alvo, Bell atualiza OOB)
    let body = format!(
        "{}{}",
        dropdown_template.render().unwrap(),
        bell_template.render().unwrap()
    );

    Ok(Html(body))
}

/// POST /admin/notifications/read-all - Marca todas como lidas
pub async fn mark_all_notifications_read_handler(
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id = get_current_user_id(&cookies, &state)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "Não autenticado".to_string()))?;

    notification_service::mark_all_read(&state.conn, user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    // Retorna novo dropdown vazio + Bell atualizado (OOB)
    let dropdown_template = NotificationsPartialTemplate {
        notifications: vec![],
        unread_count: 0,
        csrf_token,
    };

    let bell_template = NotificationBellTemplate { has_unread: false };

    let body = format!(
        "{}{}",
        dropdown_template.render().unwrap(),
        bell_template.render().unwrap()
    );

    Ok(Html(body))
}

/// GET /admin/notifications/status - Retorna status de notificações (HX-Trigger)
#[derive(Template)]
#[template(path = "admin/notification_bell.html")]
pub struct NotificationBellTemplate {
    pub has_unread: bool,
}

/// GET /admin/notifications/status - Retorna botão do sino atualizado (OOB Swap)
pub async fn check_status_handler(
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id = get_current_user_id(&cookies, &state)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "Não autenticado".to_string()))?;

    let unread_count = notification_service::count_unread(&state.conn, user_id)
        .await
        .unwrap_or(0);

    let template = NotificationBellTemplate {
        has_unread: unread_count > 0,
    };

    Ok(Html(template.render().unwrap()))
}

/// GET /admin/notifications/close - Fecha dropdown retornando vazio
pub async fn close_notifications_handler(
    State(_state): State<AppState>,
    _cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Html(""))
}
