//! Handler SSE para notificações Toast
//!
//! Fornece stream de notificações em tempo real via Server-Sent Events.

use askama::Template;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::Stream;
use std::convert::Infallible;
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use tower_cookies::Cookies;

use crate::{
    middleware::auth::get_current_user_id,
    state::{AppState, ToastLevel, ToastNotification},
};

// ============================================================================
// TEMPLATE ASKAMA
// ============================================================================

#[derive(Template)]
#[template(path = "shared/toast.html")]
struct ToastTemplate {
    level: String,
    message: String,
}

// ============================================================================
// HANDLER SSE
// ============================================================================

/// Handler SSE - stream de notificações para o usuário atual
pub async fn toast_stream_handler(
    State(state): State<AppState>,
    cookies: Cookies,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.toast_tx.subscribe();

    // Obtém user_id do usuário logado (ou None se não autenticado)
    let current_user_id = get_current_user_id(&cookies, &state).await;

    let stream = BroadcastStream::new(rx).filter_map(move |result| match result {
        Ok(toast) => {
            // Filtrar: broadcast global (None) ou para este usuário
            let should_show = toast.user_id.is_none()
                || (current_user_id.is_some() && toast.user_id == current_user_id);

            if should_show {
                let template = ToastTemplate {
                    level: toast.level.as_str().to_string(),
                    message: toast.message,
                };

                match template.render() {
                    Ok(html) => Some(Ok(Event::default().event("toast").data(html))),
                    Err(_) => None,
                }
            } else {
                None
            }
        }
        Err(_) => None, // Ignorar erros de lag
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(30))
            .text("keep-alive"),
    )
}

// ============================================================================
// HELPERS - Usar em qualquer handler
// ============================================================================

/// Envia toast para um usuário específico
pub fn send_toast(
    state: &AppState,
    user_id: uuid::Uuid,
    level: ToastLevel,
    message: impl Into<String>,
) {
    let _ = state.toast_tx.send(ToastNotification {
        level,
        message: message.into(),
        user_id: Some(user_id),
    });
}

/// Envia toast para TODOS os usuários conectados
#[allow(dead_code)]
pub fn broadcast_toast(state: &AppState, level: ToastLevel, message: impl Into<String>) {
    let _ = state.toast_tx.send(ToastNotification {
        level,
        message: message.into(),
        user_id: None,
    });
}

/// Helper simplificado que aceita Option<Uuid> (só envia se usuário existir)
pub fn toast(
    state: &AppState,
    user_id: Option<uuid::Uuid>,
    level: ToastLevel,
    message: impl Into<String>,
) {
    if let Some(uid) = user_id {
        send_toast(state, uid, level, message);
    }
}

/// DELETE /admin/toasts/dismiss - Retorna 200 OK vazio para limpar toast do DOM
pub async fn dismiss_toast_handler() -> impl axum::response::IntoResponse {
    axum::http::StatusCode::OK
}
