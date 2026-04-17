use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use std::time::Duration;

/// Inicializa a conexão com o banco de dados PostgreSQL.
///
/// Configurações do pool:
/// - 20 conexões máximas
/// - 5 conexões mínimas mantidas
/// - Timeout de conexão: 10s
/// - Timeout de aquisição: 10s
/// - Idle timeout: 5 minutos
pub async fn init(db_url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new(db_url.to_owned());

    // Pool configuration
    let sqlx_logging = std::env::var("SQLX_LOGGING")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    let sqlx_log_level = std::env::var("SQLX_LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string())
        .to_lowercase();

    let level_filter = match sqlx_log_level.as_str() {
        "debug" => tracing::log::LevelFilter::Debug,
        "warn" => tracing::log::LevelFilter::Warn,
        "error" => tracing::log::LevelFilter::Error,
        "trace" => tracing::log::LevelFilter::Trace,
        "off" => tracing::log::LevelFilter::Off,
        _ => tracing::log::LevelFilter::Info,
    };

    opt.max_connections(20)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .sqlx_logging(sqlx_logging)
        .sqlx_logging_level(level_filter);

    let conn = Database::connect(opt).await?;

    // Run migrations
    Migrator::up(&conn, None)
        .await
        .map_err(|e| DbErr::Custom(e.to_string()))?;

    tracing::info!("✅ Database connection pool initialized");

    Ok(conn)
}
