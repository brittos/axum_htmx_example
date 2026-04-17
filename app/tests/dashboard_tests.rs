//! Dashboard Integration Tests
//!
//! Tests the admin dashboard handler functionality.

use axum::{body::Body, http::StatusCode};

mod common;
use common::{login_admin, request, request_with_cookies, spawn_app_with_db};

/// Test that dashboard requires authentication
#[tokio::test]
async fn test_dashboard_requires_auth() {
    let (app, _) = spawn_app_with_db().await;

    let response = request(&app, "/admin", "GET", Body::empty()).await;

    // Should redirect to login
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("location").unwrap(), "/admin/login");
}

/// Test that authenticated user can access dashboard
#[tokio::test]
async fn test_dashboard_access_authenticated() {
    let (app, db) = spawn_app_with_db().await;

    // Login as Admin to ensure access
    let cookies = login_admin(&app, &db, "dashboard_user", "password123").await;

    let response = request_with_cookies(&app, "/admin", "GET", Body::empty(), &cookies).await;

    // Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test that dashboard returns HTML content
#[tokio::test]
async fn test_dashboard_returns_html() {
    let (app, db) = spawn_app_with_db().await;

    // Login as Admin
    let cookies = login_admin(&app, &db, "dashboard_html_user", "password123").await;

    let response = request_with_cookies(&app, "/admin", "GET", Body::empty(), &cookies).await;

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("text/html"));
}
