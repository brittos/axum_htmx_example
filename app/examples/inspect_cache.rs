//! Utilitário simples para inspecionar chaves no Redis sem precisar do redis-cli.
//!
//! Uso: cargo run -p axum-example-app --example inspect_cache

use redis::{Client, Commands};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Configuração
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    println!("🔌 Conectando ao Redis em: {}", redis_url);

    let client = Client::open(redis_url)?;
    let mut conn = client.get_connection()?;

    println!("✅ Conectado!");

    // 2. Listar todas as chaves
    let keys: Vec<String> = conn.keys("*")?;

    println!("\n📦 Chaves encontradas ({}):", keys.len());
    println!("---------------------------------------------------");

    for key in keys {
        let key_type: String = redis::cmd("TYPE").arg(&key).query(&mut conn)?;

        // Tentar obter valor baseado no tipo
        let value_display = match key_type.as_str() {
            "string" => {
                let val: String = conn.get(&key)?;
                // Truncar se for muito longo
                if val.len() > 60 {
                    format!("{}...", &val[0..57])
                } else {
                    val
                }
            }
            _ => format!("(tipo: {})", key_type),
        };

        // Ver TTL
        let ttl: i64 = conn.ttl(&key)?;

        println!("🔑 {:<30} | ⏳ {:<5}s | {}", key, ttl, value_display);
    }
    println!("---------------------------------------------------");
    println!("💡 Dica: Navegue no admin para gerar sessões e logs.");

    Ok(())
}
