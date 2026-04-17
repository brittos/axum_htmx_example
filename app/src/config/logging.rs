//! Configuração de logging estruturado com tracing.
//!
//! Logs são escritos tanto para stdout quanto para arquivos rotativos diários.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Inicializa o sistema de logging.
///
/// Configuração:
/// - Usa RUST_LOG do ambiente ou fallback para "info"
/// - Console output com cores
/// - Arquivo rotativo diário em `logs/app.log`
pub fn init() -> Option<WorkerGuard> {
    // Usar variável de ambiente existente ou fallback seguro
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let enable_file_logging = std::env::var("ENABLE_FILE_LOGGING")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    let (file_layer, guard) = if enable_file_logging {
        let file_appender = tracing_appender::rolling::daily("logs", "app.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        (
            Some(
                tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false),
            ),
            Some(guard),
        )
    } else {
        (None, None)
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level)),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(file_layer)
        .init();

    guard
}
