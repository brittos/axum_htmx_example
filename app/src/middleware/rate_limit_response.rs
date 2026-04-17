//! Middleware para customizar resposta de Rate Limit (429).
//!
//! Intercepta respostas 429 do GovernorLayer e substitui pelo template HTML estilizado.

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

use crate::handlers::error_handler::too_many_requests_response;

/// Middleware que intercepta respostas 429 e substitui pelo template HTML customizado.
///
/// Deve ser aplicado ANTES (mais externo) do GovernorLayer para poder interceptar a resposta.
pub async fn rate_limit_response_middleware(request: Request, next: Next) -> Response {
    let response = next.run(request).await;

    // Se não for 429, retorna a resposta original
    if response.status() != StatusCode::TOO_MANY_REQUESTS {
        return response;
    }

    // Extrai Retry-After header se existir
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60);

    // Retorna resposta customizada
    too_many_requests_response(retry_after)
}
