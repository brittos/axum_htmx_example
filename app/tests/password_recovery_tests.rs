use axum::{body::Body, http::StatusCode};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

mod common;
use common::{request_with_cookies, spawn_app_with_db};

#[tokio::test]
async fn test_forgot_password_success_flow() {
    let (app, conn) = spawn_app_with_db().await;

    // 1. Create a user (using login_user helper which creates if not exists)
    // We don't strictly need to login, just ensure user exists.
    // Manually create user to avoid login overhead if possible, but helper is convenient.
    let _ = common::login_user(&app, &conn, "recovery_user", "oldpassword").await;

    // 2. Request Password Reset
    // We need a CSRF cookie first. Visit the page to get it.
    let response = common::request(&app, "/auth/forgot-password", "GET", Body::empty()).await;
    let cookies: Vec<String> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string())
        .collect();
    let cookie_str = cookies.join("; ");

    let csrf_token = cookies
        .iter()
        .find(|c| c.starts_with("csrf_token="))
        .map(|c| c.trim_start_matches("csrf_token="))
        .expect("CSRF token missing");

    let body = Body::from("email=recovery_user@example.com");
    let uri = format!("/auth/forgot-password?csrf_token={}", csrf_token);
    let response = request_with_cookies(&app, &uri, "POST", body, &cookie_str).await;

    assert_eq!(response.status(), StatusCode::OK);
    // Check for success message in HTML
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    assert!(body_str.contains("receive a password reset link"));

    // 3. Verify Token in DB
    use entity::{password_resets, users};
    let user = users::Entity::find()
        .filter(users::Column::Username.eq("recovery_user"))
        .one(&conn)
        .await
        .unwrap()
        .unwrap();

    let reset_token = password_resets::Entity::find()
        .filter(password_resets::Column::UserId.eq(user.id))
        .one(&conn)
        .await
        .unwrap()
        .expect("Reset token not found in DB");

    // 4. Access Reset Page with Token
    let uri = format!("/auth/reset-password?token={}", reset_token.token);
    let response = request_with_cookies(&app, &uri, "GET", Body::empty(), &cookie_str).await;
    assert_eq!(response.status(), StatusCode::OK);

    // 5. Submit New Password
    let body = Body::from(format!(
        "token={}&password=newRecoveryPass123&confirm_password=newRecoveryPass123",
        reset_token.token
    ));
    let uri = format!("/auth/reset-password?csrf_token={}", csrf_token);
    let response = request_with_cookies(&app, &uri, "POST", body, &cookie_str).await;

    // Should redirect to login with success
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/admin/login?reset_success=true"
    );

    // 6. Verify Login with New Password
    let login_cookies =
        common::login_user(&app, &conn, "recovery_user", "newRecoveryPass123").await;
    assert!(login_cookies.contains("admin_session"));
}

#[tokio::test]
async fn test_forgot_password_unknown_email() {
    let (app, _) = spawn_app_with_db().await;

    // Get CSRF
    let response = common::request(&app, "/auth/forgot-password", "GET", Body::empty()).await;
    let cookies: Vec<String> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string())
        .collect();
    let cookie_str = cookies.join("; ");
    let csrf_token = cookies
        .iter()
        .find(|c| c.starts_with("csrf_token="))
        .map(|c| c.trim_start_matches("csrf_token="))
        .unwrap();

    let body = Body::from("email=unknown@example.com");
    let uri = format!("/auth/forgot-password?csrf_token={}", csrf_token);
    let response = request_with_cookies(&app, &uri, "POST", body, &cookie_str).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    assert!(body_str.contains("If an account exists"));
}
