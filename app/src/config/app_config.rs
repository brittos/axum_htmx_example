use dotenvy::dotenv;
use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub db_url: String,
    pub server_url: String,
    pub redis_url: String,
    // Segurança
    pub cookie_secure: bool,
    pub max_login_attempts: u32,
    pub login_lockout_minutes: u32,
}

impl AppConfig {
    pub fn load() -> Self {
        dotenv().ok();
        let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
        let host = env::var("HOST").expect("HOST is not set in .env file");
        let port = env::var("PORT").expect("PORT is not set in .env file");
        let server_url = format!("{host}:{port}");
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| {
            let host = env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string());
            let port = env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
            format!("redis://{}:{}", host, port)
        });

        // Segurança
        let cookie_secure = env::var("COOKIE_SECURE")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);
        let max_login_attempts = env::var("MAX_LOGIN_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let login_lockout_minutes = env::var("LOGIN_LOCKOUT_MINUTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15);

        Self {
            db_url,
            server_url,
            redis_url,
            cookie_secure,
            max_login_attempts,
            login_lockout_minutes,
        }
    }
}
