use redis::Client;
use redis::aio::ConnectionManager;

/// Inicializa o gerenciador de conexões do Redis.
///
/// O ConnectionManager gerencia reconexões automaticamente caso a conexão caia.
pub async fn init(redis_url: &str) -> anyhow::Result<ConnectionManager> {
    let client = Client::open(redis_url)?;

    // Testa a conexão antes de retornar o manager
    // let mut conn = client.get_multiplexed_async_connection().await?;
    // let _: () = redis::cmd("PING").query_async(&mut conn).await?;

    // Obtém o Connection Manager que trata reconexão automática
    let manager = client.get_connection_manager().await?;

    tracing::info!("✅ Redis connection initialized");

    Ok(manager)
}
