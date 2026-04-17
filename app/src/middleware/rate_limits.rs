//! Middleware de Rate Limiting.
//!
//! Configura limites de requisição para proteger a API e o login.

use governor::middleware::NoOpMiddleware;
use std::env;
use std::sync::Arc;
use tower_governor::{
    governor::{GovernorConfig, GovernorConfigBuilder},
    key_extractor::SmartIpKeyExtractor,
};

/// Configuração de Rate Limit para Login (Estrito)
/// Padrão: 5 requisições por minuto por IP.
pub fn get_login_rate_limit_config() -> Arc<GovernorConfig<SmartIpKeyExtractor, NoOpMiddleware>> {
    let limit = env::var("LOGIN_RATE_LIMIT")
        .unwrap_or_else(|_| "5".to_string())
        .parse::<u64>()
        .unwrap_or(5);

    Arc::new(
        GovernorConfigBuilder::default()
            .per_second(60) // Check interval
            .burst_size(limit as u32) // Max requests
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .unwrap(),
    )
}

/// Configuração de Rate Limit para API Geral (Relaxado)
/// Padrão: 120 requisições por minuto por IP.
pub fn get_api_rate_limit_config() -> Arc<GovernorConfig<SmartIpKeyExtractor, NoOpMiddleware>> {
    let limit = env::var("API_RATE_LIMIT")
        .unwrap_or_else(|_| "120".to_string())
        .parse::<u64>()
        .unwrap_or(120);

    // period_ms = duration to replenish 1 token.
    // To allow 'limit' requests per 60 seconds (1 minute):
    // Replenish 1 token every (60000 / limit) ms.
    let period_ms = 60_000 / limit.max(1);

    Arc::new(
        GovernorConfigBuilder::default()
            .per_millisecond(period_ms)
            .burst_size(limit as u32)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .unwrap(),
    )
}
