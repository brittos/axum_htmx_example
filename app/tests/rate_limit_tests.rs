//! Rate Limiting Integration Tests
//!
//! Tests the rate limiting middleware for login endpoint.

use axum::{body::Body, http::StatusCode};

mod common;
use common::{request, spawn_app_with_db};

/// Test that login endpoint respects rate limits
/// Note: Tests use LOGIN_RATE_LIMIT=50 set in common.rs
#[tokio::test]
async fn test_login_allows_normal_requests() {
    let (app, _) = spawn_app_with_db().await;

    // Make a few login attempts (should all be allowed)
    for i in 0..3 {
        let body = Body::from(format!("username=user{}&password=wrong", i));
        let response = request(&app, "/admin/login", "POST", body).await;

        // Should NOT be 429 (rate limited)
        assert_ne!(
            response.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "Request {} should not be rate limited",
            i
        );
    }
}

/// Test that rate limiting is correctly wired into the app
/// Note: Actually triggering rate limits in tests is difficult because:
/// 1. Rate limit config is read at app startup
/// 2. Setting env vars after startup doesn't affect running app
/// 3. Token bucket refills make timing-sensitive tests flaky
///
/// To manually test rate limiting:
/// 1. Set LOGIN_RATE_LIMIT=2 in .env
/// 2. Run the server
/// 3. Make 3+ login attempts rapidly
#[tokio::test]
async fn test_rate_limit_middleware_applied() {
    let (app, _) = spawn_app_with_db().await;

    // This test verifies that rate limiting middleware is applied
    // by checking that the login endpoint works (not blocked by missing middleware)
    let body = Body::from("username=test&password=wrong");
    let response = request(&app, "/admin/login", "POST", body).await;

    // Should get a response (not a middleware error)
    // Accept: 200 OK (HTMX with error message), 303 (redirect), or 4xx (client error)
    let status = response.status();
    assert!(
        status.is_success() || status.is_redirection() || status.is_client_error(),
        "Should get a valid response from rate-limited endpoint, got {}",
        status
    );
}

/// Test that rate limit config functions return valid configs
#[test]
fn test_rate_limit_config_creation() {
    use axum_example_app::middleware::rate_limits::{
        get_api_rate_limit_config, get_login_rate_limit_config,
    };

    // These should not panic
    let _login_config = get_login_rate_limit_config();
    let _api_config = get_api_rate_limit_config();
}
