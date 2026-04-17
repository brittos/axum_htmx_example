//! CSRF Protection Integration Tests
//!
//! Tests the CSRF middleware behavior for POST requests.

use axum::{body::Body, http::StatusCode};

mod common;
use common::{request, request_with_cookies, spawn_app_with_db};

/// Test that GET requests work without CSRF token
#[tokio::test]
async fn test_get_request_no_csrf_required() {
    let (app, _) = spawn_app_with_db().await;

    let response = request(&app, "/admin/login", "GET", Body::empty()).await;

    // Should return 200 OK (login page)
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test that POST to /admin/login works without CSRF (exempted route)
#[tokio::test]
async fn test_login_exempt_from_csrf() {
    let (app, _) = spawn_app_with_db().await;

    let body = Body::from("username=nonexistent&password=wrong");
    let response = request(&app, "/admin/login", "POST", body).await;

    // Should NOT be 403 (CSRF). It will be 303 redirect to login with error.
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}

/// Test that POST without CSRF token returns 403
#[tokio::test]
async fn test_post_without_csrf_returns_403() {
    let (app, db) = spawn_app_with_db().await;

    // Login first to get session
    let cookies = common::login_user(&app, &db, "csrf_test_user", "password123").await;

    // Try to POST to profile without CSRF token
    let body = Body::from("name=Test&email=test@test.com");
    let response = request_with_cookies(&app, "/admin/profile", "POST", body, &cookies).await;

    // Should be 403 Forbidden
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// Test that POST with valid CSRF token succeeds
#[tokio::test]
async fn test_post_with_valid_csrf_succeeds() {
    let (app, db) = spawn_app_with_db().await;

    // Login to get session + CSRF token
    let cookies = common::login_user(&app, &db, "csrf_valid_user", "password123").await;

    // Extract CSRF token from cookies
    let csrf_token = cookies
        .split(';')
        .find(|s| s.trim().starts_with("csrf_token="))
        .map(|s| s.trim().trim_start_matches("csrf_token="))
        .expect("CSRF token not found");

    // POST with CSRF token in query string
    let uri = format!("/admin/profile?csrf_token={}", csrf_token);
    let body = Body::from("name=Updated&email=updated@test.com");
    let response = request_with_cookies(&app, &uri, "POST", body, &cookies).await;

    // Should succeed (303 redirect to profile with success)
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

/// Test that POST with mismatched CSRF token returns 403
#[tokio::test]
async fn test_post_with_wrong_csrf_returns_403() {
    let (app, db) = spawn_app_with_db().await;

    // Login to get session
    let cookies = common::login_user(&app, &db, "csrf_wrong_user", "password123").await;

    // POST with fake CSRF token
    let uri = "/admin/profile?csrf_token=fake-token-12345";
    let body = Body::from("name=Hacker&email=hacker@evil.com");
    let response = request_with_cookies(&app, uri, "POST", body, &cookies).await;

    // Should be 403 Forbidden
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
