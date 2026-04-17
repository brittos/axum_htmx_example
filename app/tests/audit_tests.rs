//! Audit Logs Integration Tests
//!
//! Tests the audit logs handler functionality.

use axum::{body::Body, http::StatusCode};

mod common;
use common::{login_admin, request, request_with_cookies, spawn_app_with_db};

/// Test that audit logs page requires authentication
#[tokio::test]
async fn test_audit_logs_requires_auth() {
    let (app, _) = spawn_app_with_db().await;

    let response = request(&app, "/admin/audit-logs", "GET", Body::empty()).await;

    // Should redirect to login
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("location").unwrap(), "/admin/login");
}

/// Test that authenticated user can access audit logs
#[tokio::test]
async fn test_audit_logs_access_authenticated() {
    let (app, db) = spawn_app_with_db().await;

    // Login
    let cookies = login_admin(&app, &db, "audit_user", "password123").await;

    let response =
        request_with_cookies(&app, "/admin/audit-logs", "GET", Body::empty(), &cookies).await;

    // Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test audit logs pagination
#[tokio::test]
async fn test_audit_logs_pagination() {
    let (app, db) = spawn_app_with_db().await;

    let cookies = login_admin(&app, &db, "audit_page_user", "password123").await;

    // Request page 1
    let response = request_with_cookies(
        &app,
        "/admin/audit-logs?page=1",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);

    // Request page 2
    let response = request_with_cookies(
        &app,
        "/admin/audit-logs?page=2",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

/// Test audit logs filtering by action
#[tokio::test]
async fn test_audit_logs_filter_by_action() {
    let (app, db) = spawn_app_with_db().await;

    let cookies = login_admin(&app, &db, "audit_filter_user", "password123").await;

    let response = request_with_cookies(
        &app,
        "/admin/audit-logs?action=create",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

/// Test audit logs filtering by entity type
#[tokio::test]
async fn test_audit_logs_filter_by_entity_type() {
    let (app, db) = spawn_app_with_db().await;

    let cookies = login_admin(&app, &db, "audit_entity_user", "password123").await;

    let response = request_with_cookies(
        &app,
        "/admin/audit-logs?entity_type=user",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

/// Test audit logs filtering by username (text search)
#[tokio::test]
async fn test_audit_logs_filter_by_username() {
    let (app, db) = spawn_app_with_db().await;

    let cookies = login_admin(&app, &db, "audit_username_user", "password123").await;

    // Filter by username partial match
    let response = request_with_cookies(
        &app,
        "/admin/audit-logs?username=audit",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

/// Test audit logs filtering by date range
#[tokio::test]
async fn test_audit_logs_filter_by_date_range() {
    let (app, db) = spawn_app_with_db().await;

    let cookies = login_admin(&app, &db, "audit_date_user", "password123").await;

    // Filter by date range
    let response = request_with_cookies(
        &app,
        "/admin/audit-logs?date_from=2024-01-01&date_to=2025-12-31",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

/// Test audit logs CSV export requires authentication
#[tokio::test]
async fn test_audit_logs_csv_export_requires_auth() {
    let (app, _) = spawn_app_with_db().await;

    let response = request(&app, "/admin/audit-logs/export-csv", "GET", Body::empty()).await;

    // Should redirect to login
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("location").unwrap(), "/admin/login");
}

/// Test audit logs CSV export returns CSV content
#[tokio::test]
async fn test_audit_logs_csv_export() {
    let (app, db) = spawn_app_with_db().await;

    let cookies = login_admin(&app, &db, "audit_csv_user", "password123").await;

    let response = request_with_cookies(
        &app,
        "/admin/audit-logs/export-csv",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);

    // Check content type
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("text/csv"));

    // Check content disposition
    let content_disposition = response.headers().get("content-disposition").unwrap();
    assert!(content_disposition.to_str().unwrap().contains("attachment"));
}

/// Test audit logs CSV export with filters
#[tokio::test]
async fn test_audit_logs_csv_export_with_filters() {
    let (app, db) = spawn_app_with_db().await;

    let cookies = login_admin(&app, &db, "audit_csv_filter_user", "password123").await;

    // Export with filters
    let response = request_with_cookies(
        &app,
        "/admin/audit-logs/export-csv?action=login&entity_type=user",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/csv")
    );
}

/// Test combined filters work together
#[tokio::test]
async fn test_audit_logs_combined_filters() {
    let (app, db) = spawn_app_with_db().await;

    let cookies = login_admin(&app, &db, "audit_combo_user", "password123").await;

    let response = request_with_cookies(
        &app,
        "/admin/audit-logs?action=create&entity_type=user&date_from=2024-01-01",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}
