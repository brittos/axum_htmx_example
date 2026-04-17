//! Serviço para gerenciamento de notificações in-app.
//!
//! Este módulo fornece funções para criar, listar e gerenciar
//! notificações para usuários do dashboard.

use chrono::Utc;
use entity::notifications;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;

/// Tipos de notificação disponíveis
#[derive(Debug, Clone, Copy)]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// Cria uma nova notificação para um usuário.
pub async fn create(
    db: &DatabaseConnection,
    user_id: Uuid,
    title: &str,
    message: &str,
    notification_type: NotificationType,
    action_url: Option<&str>,
) -> Result<notifications::Model, DbErr> {
    let notification = notifications::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user_id),
        title: Set(title.to_string()),
        message: Set(message.to_string()),
        notification_type: Set(notification_type.as_str().to_string()),
        is_read: Set(false),
        action_url: Set(action_url.map(|s| s.to_string())),
        created_at: Set(Utc::now().fixed_offset()),
    };

    notification.insert(db).await
}

/// Lista notificações de um usuário (ordenadas por data, mais recentes primeiro).
pub async fn list_for_user(
    db: &DatabaseConnection,
    user_id: Uuid,
    limit: u64,
) -> Result<Vec<notifications::Model>, DbErr> {
    notifications::Entity::find()
        .filter(notifications::Column::UserId.eq(user_id))
        .order_by_desc(notifications::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
}

/// Lista notificações não lidas de um usuário.
pub async fn list_unread(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<notifications::Model>, DbErr> {
    notifications::Entity::find()
        .filter(notifications::Column::UserId.eq(user_id))
        .filter(notifications::Column::IsRead.eq(false))
        .order_by_desc(notifications::Column::CreatedAt)
        .all(db)
        .await
}

/// Conta notificações não lidas de um usuário.
pub async fn count_unread(db: &DatabaseConnection, user_id: Uuid) -> Result<u64, DbErr> {
    use sea_orm::PaginatorTrait;

    notifications::Entity::find()
        .filter(notifications::Column::UserId.eq(user_id))
        .filter(notifications::Column::IsRead.eq(false))
        .count(db)
        .await
}

/// Marca uma notificação como lida (apenas se pertencer ao usuário).
pub async fn mark_as_read(
    db: &DatabaseConnection,
    notification_id: Uuid,
    user_id: Uuid,
) -> Result<(), DbErr> {
    if let Some(notification) = notifications::Entity::find_by_id(notification_id)
        .filter(notifications::Column::UserId.eq(user_id))
        .one(db)
        .await?
    {
        let mut active: notifications::ActiveModel = notification.into();
        active.is_read = Set(true);
        active.update(db).await?;
    }

    Ok(())
}

/// Marca todas as notificações de um usuário como lidas.
pub async fn mark_all_read(db: &DatabaseConnection, user_id: Uuid) -> Result<u64, DbErr> {
    use sea_orm::sea_query::Expr;

    let result = notifications::Entity::update_many()
        .col_expr(notifications::Column::IsRead, Expr::value(true))
        .filter(notifications::Column::UserId.eq(user_id))
        .filter(notifications::Column::IsRead.eq(false))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

/// Deleta notificações antigas (lidas, mais velhas que X dias).
pub async fn cleanup_old_notifications(
    db: &DatabaseConnection,
    older_than_days: i64,
) -> Result<u64, DbErr> {
    use chrono::Duration;

    let cutoff = Utc::now().fixed_offset() - Duration::days(older_than_days);

    let result = notifications::Entity::delete_many()
        .filter(notifications::Column::IsRead.eq(true))
        .filter(notifications::Column::CreatedAt.lt(cutoff))
        .exec(db)
        .await?;

    if result.rows_affected > 0 {
        tracing::info!(
            "Cleaned up {} old notification records",
            result.rows_affected
        );
    }

    Ok(result.rows_affected)
}
