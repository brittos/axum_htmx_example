mod common;
use axum::body::Body;
use axum_example_app::utils::password::hash_password;
use common::{TestApp, request, setup};
use entity::users;
use sea_orm::{ActiveModelTrait, Set};

#[tokio::test]
async fn test_login_success() {
    let TestApp { app, conn, .. } = setup().await;

    // Create Test User
    let password = "password123";
    let hash = hash_password(password).unwrap();

    let user = users::ActiveModel {
        id: Set(uuid::Uuid::now_v7()),
        name: Set("Test User".to_string()),
        username: Set("testuser".to_string()),
        email: Set("test@example.com".to_string()),
        password: Set(hash),
        status: Set("Active".to_string()),
        is_banned: Set(false),
        ..Default::default()
    };
    user.insert(&conn).await.unwrap();

    // Perform Login Request
    let body = Body::from("username=testuser&password=password123");
    let response = request(&app, "/admin/login", "POST", body).await;

    // HTMX usa HX-Redirect header em vez de Location
    let hx_redirect = response.headers().get("hx-redirect");
    assert!(
        hx_redirect.is_some(),
        "HX-Redirect header not found. Headers: {:?}",
        response.headers()
    );
    assert_eq!(hx_redirect.unwrap(), "/admin");

    let cookies: Vec<String> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect();

    let has_session = cookies.iter().any(|c| c.contains("admin_session"));
    assert!(
        has_session,
        "Session cookie not found in headers: {:?}",
        cookies
    );
}

#[tokio::test]
async fn test_login_fail_wrong_password() {
    let TestApp { app, conn, .. } = setup().await;

    // Create Test User
    let hash = hash_password("password123").unwrap();

    let user = users::ActiveModel {
        id: Set(uuid::Uuid::now_v7()),
        name: Set("Test User".to_string()),
        username: Set("testuser".to_string()),
        email: Set("test@example.com".to_string()),
        password: Set(hash),
        status: Set("Active".to_string()),
        is_banned: Set(false),
        ..Default::default()
    };
    user.insert(&conn).await.unwrap();

    // Perform Login Request
    let body = Body::from("username=testuser&password=wrongpassword");
    let response = request(&app, "/admin/login", "POST", body).await;

    // HTMX: erro retorna headers HX-Retarget + HTML
    assert!(response.headers().get("hx-retarget").is_some());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        body_str.contains("alert--error"),
        "Expected error HTML, got: {}",
        body_str
    );
}

#[tokio::test]
async fn test_login_fail_unknown_user() {
    let TestApp { app, .. } = setup().await;

    // Perform Login Request
    let body = Body::from("username=ghost&password=123");
    let response = request(&app, "/admin/login", "POST", body).await;

    // HTMX: erro retorna headers HX-Retarget + HTML
    assert!(response.headers().get("hx-retarget").is_some());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("Usuário ou senha inválidos"));
}
