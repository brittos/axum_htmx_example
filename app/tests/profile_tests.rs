use axum::{body::Body, http::StatusCode};

mod common;
use common::{login_user, request_with_cookies, spawn_app_with_db};

#[tokio::test]
async fn test_profile_access_protected() {
    let (app, _) = spawn_app_with_db().await;

    // Use common::request to ensure ConnectInfo is present (required by RateLimiter)
    let body = Body::empty();
    let response = common::request(&app, "/admin/profile", "GET", body).await;

    // Sem login -> 303 Redirect para login
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("location").unwrap(), "/admin/login");
}

#[tokio::test]
async fn test_profile_update_info_and_password() {
    let (app, db) = spawn_app_with_db().await;
    let cookies = login_user(&app, &db, "test_profile_user", "password123").await;

    // Helper to extract CSRF token from cookies
    let csrf_token = cookies
        .split(';')
        .find(|s| s.trim().starts_with("csrf_token="))
        .map(|s| s.trim().trim_start_matches("csrf_token="))
        .expect("CSRF token not found in cookies");

    // 1. Update Profile Info
    let body = Body::from("name=Admin+Updated&email=admin_updated@example.com");
    // Append csrf_token to URL query string as required by our profile_content.html fix
    let uri = format!("/admin/profile?csrf_token={}", csrf_token);

    let response = request_with_cookies(&app, &uri, "POST", body, &cookies).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/admin/profile?success=true"
    );

    // 2. Change Password (Fail - Mismatch)
    let body =
        Body::from("current_password=admin&new_password=newpass123&confirm_password=mismatch");
    let uri = format!("/admin/profile/password?csrf_token={}", csrf_token);

    let response = request_with_cookies(&app, &uri, "POST", body, &cookies).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.contains("error=As+senhas+n%C3%A3o+coincidem"));

    // 3. Change Password (Fail - Wrong Current)
    let body =
        Body::from("current_password=wrong&new_password=newpass123&confirm_password=newpass123");
    let uri = format!("/admin/profile/password?csrf_token={}", csrf_token);

    let response = request_with_cookies(&app, &uri, "POST", body, &cookies).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.contains("error=Senha+incorreta"));

    // 4. Change Password (Success)
    let body = Body::from(
        "current_password=password123&new_password=newpass123&confirm_password=newpass123",
    );
    let uri = format!("/admin/profile/password?csrf_token={}", csrf_token);

    let response = request_with_cookies(&app, &uri, "POST", body, &cookies).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.contains("success=true"));

    // 5. Verify Login with New Password
    let new_login_cookies = login_user(&app, &db, "test_profile_user", "newpass123").await;
    assert!(new_login_cookies.contains("admin_session"));
}
