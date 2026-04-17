//! Test helpers and utilities shared across integration tests.
//!
//! Note: The `dead_code` warning is suppressed because each test file is compiled
//! as a separate crate, causing Rust to incorrectly report these shared helpers as unused.
#![allow(dead_code)]

use axum::extract::ConnectInfo;
use axum::{
    Router,
    body::Body,
    http::{Request, Response, StatusCode},
};
use std::net::SocketAddr;

use axum_example_app::state::AppState;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use tower::ServiceExt;

pub fn get_random_email() -> String {
    format!("{}@example.com", uuid::Uuid::now_v7())
}

pub struct TestApp {
    pub app: Router,
    pub conn: DatabaseConnection,
    pub redis: redis::aio::ConnectionManager,
}

pub async fn setup() -> TestApp {
    unsafe {
        std::env::set_var("LOGIN_RATE_LIMIT", "50");
    }

    // Setup tracing/logging only once used commonly in tests
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .try_init();

    // In-memory SQLite DB
    let db_url = "sqlite::memory:";

    let mut opt = ConnectOptions::new(db_url.to_owned());
    opt.sqlx_logging(true); // Enable SQL logs debugging

    let conn = Database::connect(opt)
        .await
        .expect("Failed to connect to in-memory DB");

    // Run Migrations
    Migrator::up(&conn, None)
        .await
        .expect("Failed to run migrations");

    // Seed data if needed (admin user is seeded by migrations usually, but we check)

    // Redis for tests
    let redis_url = "redis://127.0.0.1:6379";
    let redis = axum_example_app::config::redis::init(redis_url)
        .await
        .expect("Failed to connect to Redis for testing");

    // Canal de broadcast para toasts (testes)
    let (toast_tx, _) =
        tokio::sync::broadcast::channel::<axum_example_app::state::ToastNotification>(100);

    // Config para testes
    let config = axum_example_app::config::AppConfig {
        db_url: db_url.to_string(),
        server_url: "127.0.0.1:8000".to_string(),
        redis_url: redis_url.to_string(),
        cookie_secure: false,
        max_login_attempts: 5,
        login_lockout_minutes: 15,
    };

    let state = AppState {
        conn: conn.clone(),
        redis: redis.clone(),
        toast_tx,
        config,
    };

    let app = axum_example_app::routes::configure_routes(state);

    TestApp { app, conn, redis }
}

pub async fn spawn_app_with_db() -> (Router, DatabaseConnection) {
    let app = setup().await;
    (app.app, app.conn)
}

pub async fn request(app: &Router, uri: &str, method: &str, body: Body) -> Response<Body> {
    request_with_cookies(app, uri, method, body, "").await
}

pub async fn request_with_cookies(
    app: &Router,
    uri: &str,
    method: &str,
    body: Body,
    cookies: &str,
) -> Response<Body> {
    let mut builder = Request::builder()
        .uri(uri)
        .method(method)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 8080))));

    if !cookies.is_empty() {
        builder = builder.header("Cookie", cookies);
    }

    app.clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

pub async fn login_user(
    app: &Router,
    conn: &DatabaseConnection,
    username: &str,
    password: &str,
) -> String {
    use axum_example_app::utils::password::hash_password;
    use entity::users;
    use sea_orm::{ActiveModelTrait, Set};

    // 1. Ensure user exists
    let hash = hash_password(password).unwrap();
    let user = users::ActiveModel {
        id: Set(uuid::Uuid::now_v7()),
        name: Set("Test User".to_string()),
        username: Set(username.to_string()),
        email: Set(format!("{}@example.com", username)),
        password: Set(hash),
        status: Set("Active".to_string()),
        is_banned: Set(false),
        ..Default::default()
    };
    // Ignore error if user already exists (assume success or handle duplication if needed)
    let _ = user.insert(conn).await;

    // 2. Perform Login
    let body = Body::from(format!("username={}&password={}", username, password));
    let response = request(app, "/admin/login", "POST", body).await;

    // Aceitar tanto SEE_OTHER (redirect HTTP) quanto 200 OK com HX-Redirect (HTMX)
    let is_redirect = response.status() == StatusCode::SEE_OTHER;
    let is_htmx_redirect =
        response.status() == StatusCode::OK && response.headers().contains_key("HX-Redirect");

    if !is_redirect && !is_htmx_redirect {
        panic!(
            "Login failed for user {}: status {}",
            username,
            response.status()
        );
    }

    // 3. Extract Cookies (name=value only)
    let cookies: Vec<String> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| {
            let s = v.to_str().unwrap();
            s.split(';').next().unwrap_or("").to_string()
        })
        .collect();

    cookies.join("; ")
}

pub async fn login_admin(
    app: &Router,
    conn: &DatabaseConnection,
    username: &str,
    password: &str,
) -> String {
    use axum_example_app::utils::password::hash_password;
    use entity::{roles, user_roles, users};
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

    // 1. Ensure user exists
    let hash = hash_password(password).unwrap();
    let user_id = uuid::Uuid::now_v7();

    // Check if user exists first to avoid conflict
    let user_model = if let Ok(Some(u)) = users::Entity::find()
        .filter(users::Column::Username.eq(username))
        .one(conn)
        .await
    {
        u
    } else {
        let user = users::ActiveModel {
            id: Set(user_id),
            name: Set("Admin Test User".to_string()),
            username: Set(username.to_string()),
            email: Set(format!("{}@example.com", username)),
            password: Set(hash),
            status: Set("Active".to_string()),
            is_banned: Set(false),
            ..Default::default()
        };
        user.insert(conn)
            .await
            .expect("Failed to create admin user")
    };

    // 2. Ensure Admin role exists
    let admin_role = roles::Entity::find()
        .filter(roles::Column::Name.eq("Admin"))
        .one(conn)
        .await
        .expect("Failed to query roles");

    let role_id = if let Some(role) = admin_role {
        role.id
    } else {
        // Create Admin role if not exists
        let role = roles::ActiveModel {
            id: Set(uuid::Uuid::now_v7()),
            name: Set("Admin".to_string()),
        };
        role.insert(conn)
            .await
            .expect("Failed to create Admin role")
            .id
    };

    // 3. Assign Admin role to user
    // Delete existing roles first
    let _ = user_roles::Entity::delete_many()
        .filter(user_roles::Column::UserId.eq(user_model.id))
        .exec(conn)
        .await;

    let user_role = user_roles::ActiveModel {
        user_id: Set(user_model.id),
        role_id: Set(role_id),
    };
    user_role
        .insert(conn)
        .await
        .expect("Failed to assign admin role");

    // 4. Perform Login
    let body = Body::from(format!("username={}&password={}", username, password));
    let response = request(app, "/admin/login", "POST", body).await;

    // Aceitar tanto SEE_OTHER (redirect HTTP) quanto 200 OK com HX-Redirect (HTMX)
    let is_redirect = response.status() == StatusCode::SEE_OTHER;
    let is_htmx_redirect =
        response.status() == StatusCode::OK && response.headers().contains_key("HX-Redirect");

    if !is_redirect && !is_htmx_redirect {
        panic!(
            "Login failed for admin {}: status {}",
            username,
            response.status()
        );
    }

    // 5. Extract Cookies
    let cookies: Vec<String> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| {
            let s = v.to_str().unwrap();
            s.split(';').next().unwrap_or("").to_string()
        })
        .collect();

    cookies.join("; ")
}

pub async fn spawn_app_full() -> TestApp {
    setup().await
}

pub async fn login_user_with_role(
    app: &Router,
    conn: &DatabaseConnection,
    username: &str,
    password: &str,
    role_name: &str,
) -> String {
    use axum_example_app::utils::password::hash_password;
    use entity::{roles, user_roles, users};
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

    // 1. Ensure user exists
    let hash = hash_password(password).unwrap();
    let user_id = uuid::Uuid::now_v7();

    let user_model = if let Ok(Some(u)) = users::Entity::find()
        .filter(users::Column::Username.eq(username))
        .one(conn)
        .await
    {
        u
    } else {
        let user = users::ActiveModel {
            id: Set(user_id),
            name: Set(format!("Test User {}", role_name)),
            username: Set(username.to_string()),
            email: Set(format!("{}@example.com", username)),
            password: Set(hash),
            status: Set("Active".to_string()),
            is_banned: Set(false),
            ..Default::default()
        };
        user.insert(conn).await.expect("Failed to create user")
    };

    // 2. Ensure Role exists
    let role_model = if let Ok(Some(r)) = roles::Entity::find()
        .filter(roles::Column::Name.eq(role_name))
        .one(conn)
        .await
    {
        r
    } else {
        let role = roles::ActiveModel {
            id: Set(uuid::Uuid::now_v7()),
            name: Set(role_name.to_string()),
        };
        role.insert(conn).await.expect("Failed to create role")
    };

    // 3. Assign role to user
    let _ = user_roles::Entity::delete_many()
        .filter(user_roles::Column::UserId.eq(user_model.id))
        .exec(conn)
        .await;

    let user_role = user_roles::ActiveModel {
        user_id: Set(user_model.id),
        role_id: Set(role_model.id),
    };
    user_role.insert(conn).await.expect("Failed to assign role");

    // 4. Perform Login
    let body = Body::from(format!("username={}&password={}", username, password));
    let response = request(app, "/admin/login", "POST", body).await;

    // Accept both SEE_OTHER (HTTP redirect) and 200 OK with HX-Redirect (HTMX)
    let is_redirect = response.status() == StatusCode::SEE_OTHER;
    let is_htmx_redirect =
        response.status() == StatusCode::OK && response.headers().contains_key("HX-Redirect");

    if !is_redirect && !is_htmx_redirect {
        panic!("Login failed: status {}", response.status());
    }

    let cookies: Vec<String> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| {
            let s = v.to_str().unwrap();
            s.split(';').next().unwrap_or("").to_string()
        })
        .collect();

    cookies.join("; ")
}
