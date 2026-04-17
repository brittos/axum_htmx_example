use crate::config::AppConfig;
use sea_orm::DatabaseConnection;
use tokio::sync::broadcast;

/// Níveis de notificação toast
#[derive(Clone, Debug)]
pub enum ToastLevel {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

/// Estrutura de uma notificação toast
#[derive(Clone, Debug)]
pub struct ToastNotification {
    pub level: ToastLevel,
    pub message: String,
    pub user_id: Option<uuid::Uuid>, // None = broadcast para todos
}

#[derive(Clone)]
pub struct AppState {
    pub conn: DatabaseConnection,
    pub redis: redis::aio::ConnectionManager,
    pub toast_tx: broadcast::Sender<ToastNotification>,
    pub config: AppConfig,
}
