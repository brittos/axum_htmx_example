//! Serviço para gerenciamento de sessões.
//!
//! Este módulo fornece funções para limpeza de sessões expiradas
//! e outras operações de manutenção.

use entity::sessions;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

/// Resultado da limpeza de sessões
#[derive(Debug)]
pub struct CleanupResult {
    /// Número de sessões removidas
    pub deleted_count: u64,
    /// Erros encontrados (se houver)
    pub errors: Vec<String>,
}

/// Remove todas as sessões expiradas do banco de dados.
///
/// Esta função deve ser chamada periodicamente (via cron job ou scheduler)
/// para evitar acúmulo de sessões inválidas.
///
/// # Exemplo
/// ```rust,no_run
/// use axum_example_app::service::session_service::cleanup_expired_sessions;
/// use sea_orm::DatabaseConnection;
///
/// async fn cleanup_job(db: &DatabaseConnection) {
///     match cleanup_expired_sessions(db).await {
///         Ok(result) => println!("Removed {} expired sessions", result.deleted_count),
///         Err(e) => eprintln!("Cleanup failed: {}", e),
///     }
/// }
/// ```
pub async fn cleanup_expired_sessions(db: &DatabaseConnection) -> Result<CleanupResult, DbErr> {
    let now = chrono::Utc::now().fixed_offset();

    // Deletar todas as sessões onde expires_at < now
    let result = sessions::Entity::delete_many()
        .filter(sessions::Column::ExpiresAt.lt(now))
        .exec(db)
        .await?;

    let deleted_count = result.rows_affected;

    if deleted_count > 0 {
        tracing::info!(
            "Session cleanup completed: {} expired sessions removed",
            deleted_count
        );
    } else {
        tracing::debug!("Session cleanup: no expired sessions found");
    }

    Ok(CleanupResult {
        deleted_count,
        errors: Vec::new(),
    })
}

/// Remove todas as sessões de um usuário específico.
///
/// Útil para implementar "logout de todos os dispositivos".
///
/// # Argumentos
/// * `db` - Conexão com o banco de dados
/// * `user_id` - ID do usuário
///
/// # Retorna
/// Número de sessões removidas
pub async fn invalidate_all_user_sessions(
    db: &DatabaseConnection,
    user_id: uuid::Uuid,
) -> Result<u64, DbErr> {
    let result = sessions::Entity::delete_many()
        .filter(sessions::Column::UserId.eq(user_id))
        .exec(db)
        .await?;

    tracing::info!(
        "Invalidated {} sessions for user {}",
        result.rows_affected,
        user_id
    );

    Ok(result.rows_affected)
}

/// Remove sessões expiradas e também limpa do Redis.
///
/// Esta versão mais completa também remove as chaves de cache do Redis.
pub async fn cleanup_expired_sessions_with_redis(
    db: &DatabaseConnection,
    redis: &mut redis::aio::ConnectionManager,
) -> Result<CleanupResult, DbErr> {
    use redis::AsyncCommands;

    let now = chrono::Utc::now().fixed_offset();
    let mut errors = Vec::new();

    // 1. Buscar sessões expiradas para obter os tokens
    let expired_sessions = sessions::Entity::find()
        .filter(sessions::Column::ExpiresAt.lt(now))
        .all(db)
        .await?;

    // 2. Deletar do Redis
    for session in &expired_sessions {
        let redis_key = format!("session:{}", session.token);
        if let Err(e) = redis.del::<_, ()>(&redis_key).await {
            errors.push(format!("Redis delete failed for {}: {}", redis_key, e));
        }
    }

    // 3. Deletar do banco de dados
    let result = sessions::Entity::delete_many()
        .filter(sessions::Column::ExpiresAt.lt(now))
        .exec(db)
        .await?;

    let deleted_count = result.rows_affected;

    if deleted_count > 0 {
        tracing::info!(
            "Session cleanup completed: {} expired sessions removed (Redis errors: {})",
            deleted_count,
            errors.len()
        );
    }

    Ok(CleanupResult {
        deleted_count,
        errors,
    })
}

/// Conta o número de sessões ativas para um usuário.
pub async fn count_active_sessions(
    db: &DatabaseConnection,
    user_id: uuid::Uuid,
) -> Result<u64, DbErr> {
    use sea_orm::PaginatorTrait;

    sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(user_id))
        .filter(sessions::Column::ExpiresAt.gt(chrono::Utc::now().fixed_offset()))
        .count(db)
        .await
}

/// Lista as sessões ativas de um usuário.
pub async fn list_active_sessions(
    db: &DatabaseConnection,
    user_id: uuid::Uuid,
) -> Result<Vec<sessions::Model>, DbErr> {
    use sea_orm::QueryOrder;

    sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(user_id))
        .filter(sessions::Column::ExpiresAt.gt(chrono::Utc::now().fixed_offset()))
        .order_by_desc(sessions::Column::CreatedAt)
        .all(db)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_result_default() {
        let result = CleanupResult {
            deleted_count: 0,
            errors: Vec::new(),
        };
        assert_eq!(result.deleted_count, 0);
        assert!(result.errors.is_empty());
    }
}
