pub mod config;
pub mod dto;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod repository;
pub mod routes;
pub mod service;
pub mod state;
pub mod utils;

// Embedded assets only compiled in release mode
#[cfg(not(debug_assertions))]
pub mod embedded_assets;

use config::AppConfig;
use sea_orm::DatabaseConnection;
use state::AppState;
use std::time::Duration;
use tokio::sync::broadcast;

/// Intervalo padrão para limpeza de sessões (1 hora)
const SESSION_CLEANUP_INTERVAL_SECS: u64 = 3600;

/// Inicia o background job para limpeza de sessões expiradas.
///
/// Este job roda em uma task separada e executa periodicamente
/// a limpeza de sessões expiradas do banco de dados.
fn spawn_session_cleanup_job(conn: DatabaseConnection, redis: redis::aio::ConnectionManager) {
    let interval_secs = std::env::var("SESSION_CLEANUP_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(SESSION_CLEANUP_INTERVAL_SECS);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

        // Skip primeiro tick imediato
        interval.tick().await;

        tracing::info!(
            "🧹 Session cleanup job started (interval: {}s)",
            interval_secs
        );

        loop {
            interval.tick().await;

            tracing::debug!("Running session cleanup...");

            let mut redis_conn = redis.clone();
            match service::session_service::cleanup_expired_sessions_with_redis(
                &conn,
                &mut redis_conn,
            )
            .await
            {
                Ok(result) => {
                    if result.deleted_count > 0 {
                        tracing::info!(
                            "🧹 Session cleanup: {} expired sessions removed",
                            result.deleted_count
                        );
                    }
                    if !result.errors.is_empty() {
                        tracing::warn!("Session cleanup Redis errors: {:?}", result.errors);
                    }
                }
                Err(e) => {
                    tracing::error!("Session cleanup failed: {}", e);
                }
            }
        }
    });
}

#[tokio::main]
async fn start() -> anyhow::Result<()> {
    // Initialize logging
    let _guard = config::logging::init();

    let config = AppConfig::load();

    let conn = config::db::init(&config.db_url).await?;
    let redis = config::redis::init(&config.redis_url).await?;

    // Iniciar job de limpeza de sessões em background
    spawn_session_cleanup_job(conn.clone(), redis.clone());

    // Canal de broadcast para toasts (capacidade: 100 mensagens)
    let (toast_tx, _) = broadcast::channel::<state::ToastNotification>(100);

    let server_url = config.server_url.clone();
    let state = AppState {
        conn,
        redis,
        toast_tx,
        config,
    };

    let app = routes::configure_routes(state);

    let listener = tokio::net::TcpListener::bind(&server_url).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tracing::info!("✅ Server running successfully");
    tracing::info!("🚀 listening on {}", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}

pub fn main() {
    let result = start();

    if let Some(err) = result.err() {
        println!("Error: {err}");
    }
}
