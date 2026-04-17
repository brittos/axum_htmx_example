//! Serviço para gerenciamento de tentativas de login.
//!
//! Este módulo fornece funções para verificar, registrar e bloquear
//! tentativas de login baseado em username/IP.

use chrono::{Duration, Utc};
use entity::login_attempts;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};

/// Resultado da verificação de tentativa de login
#[derive(Debug)]
pub enum LoginAttemptResult {
    /// Pode tentar login
    Allowed,
    /// Bloqueado, com minutos restantes
    Blocked { minutes_remaining: i64 },
}

/// Verifica se o username/IP pode tentar login.
///
/// Retorna `Allowed` se pode tentar, ou `Blocked` com minutos restantes.
pub async fn can_attempt_login(
    db: &DatabaseConnection,
    username: &str,
    ip_address: &str,
    max_attempts: u32,
) -> Result<LoginAttemptResult, DbErr> {
    let now = Utc::now().fixed_offset();

    // Buscar registro existente
    let record = login_attempts::Entity::find()
        .filter(login_attempts::Column::Username.eq(username))
        .filter(login_attempts::Column::IpAddress.eq(ip_address))
        .one(db)
        .await?;

    match record {
        Some(attempt) => {
            // Verificar se está bloqueado
            if let Some(locked_until) = attempt.locked_until
                && locked_until > now
            {
                let minutes = (locked_until - now).num_minutes();
                return Ok(LoginAttemptResult::Blocked {
                    minutes_remaining: if minutes < 1 { 1 } else { minutes },
                });
            }

            // Verificar se atingiu limite
            if attempt.attempts >= max_attempts as i32 {
                // Deveria estar bloqueado mas locked_until expirou
                // Resetar para permitir nova tentativa
                Ok(LoginAttemptResult::Allowed)
            } else {
                Ok(LoginAttemptResult::Allowed)
            }
        }
        None => Ok(LoginAttemptResult::Allowed),
    }
}

/// Registra uma tentativa de login falha.
///
/// Incrementa o contador e bloqueia se atingir o limite.
pub async fn record_failed_attempt(
    db: &DatabaseConnection,
    username: &str,
    ip_address: &str,
    max_attempts: u32,
    lockout_minutes: u32,
) -> Result<Option<i64>, DbErr> {
    let now = Utc::now().fixed_offset();

    // Buscar registro existente
    let record = login_attempts::Entity::find()
        .filter(login_attempts::Column::Username.eq(username))
        .filter(login_attempts::Column::IpAddress.eq(ip_address))
        .one(db)
        .await?;

    match record {
        Some(attempt) => {
            let new_attempts = attempt.attempts + 1;
            let locked_until = if new_attempts >= max_attempts as i32 {
                Some(now + Duration::minutes(lockout_minutes as i64))
            } else {
                None
            };

            let mut active: login_attempts::ActiveModel = attempt.into();
            active.attempts = Set(new_attempts);
            active.locked_until = Set(locked_until);
            active.updated_at = Set(now);
            active.update(db).await?;

            Ok(locked_until.map(|lu| (lu - now).num_minutes()))
        }
        None => {
            // Criar novo registro
            let new_attempts = 1;
            let locked_until = if new_attempts >= max_attempts as i32 {
                Some(now + Duration::minutes(lockout_minutes as i64))
            } else {
                None
            };

            let new_record = login_attempts::ActiveModel {
                id: Set(uuid::Uuid::now_v7()),
                username: Set(username.to_string()),
                ip_address: Set(ip_address.to_string()),
                attempts: Set(new_attempts),
                locked_until: Set(locked_until),
                created_at: Set(now),
                updated_at: Set(now),
            };
            new_record.insert(db).await?;

            Ok(locked_until.map(|lu| (lu - now).num_minutes()))
        }
    }
}

/// Limpa tentativas após login bem-sucedido.
pub async fn clear_attempts(
    db: &DatabaseConnection,
    username: &str,
    ip_address: &str,
) -> Result<(), DbErr> {
    login_attempts::Entity::delete_many()
        .filter(login_attempts::Column::Username.eq(username))
        .filter(login_attempts::Column::IpAddress.eq(ip_address))
        .exec(db)
        .await?;

    Ok(())
}

/// Limpa registros antigos de tentativas de login.
/// Deve ser chamado periodicamente para limpeza.
pub async fn cleanup_old_attempts(
    db: &DatabaseConnection,
    older_than_hours: i64,
) -> Result<u64, DbErr> {
    let cutoff = Utc::now().fixed_offset() - Duration::hours(older_than_hours);

    let result = login_attempts::Entity::delete_many()
        .filter(login_attempts::Column::UpdatedAt.lt(cutoff))
        .exec(db)
        .await?;

    if result.rows_affected > 0 {
        tracing::info!(
            "Cleaned up {} old login attempt records",
            result.rows_affected
        );
    }

    Ok(result.rows_affected)
}
