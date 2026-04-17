mod common;
use axum::{body::Body, http::StatusCode};
use common::{TestApp, setup};
use entity::{roles, sessions, users};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tower::ServiceExt;
use uuid::Uuid;

async fn create_admin_session(conn: &sea_orm::DatabaseConnection) -> String {
    // Create Admin User
    let user_id = Uuid::now_v7();
    let user = users::ActiveModel {
        id: Set(user_id),
        name: Set("Admin User".to_string()),
        username: Set("admin".to_string()),
        email: Set("admin@example.com".to_string()),
        password: Set("hash".to_string()),
        status: Set("Active".to_string()),
        is_banned: Set(false),
        ..Default::default()
    };
    user.insert(conn).await.unwrap();

    // Assign Admin Role (Required for RBAC)
    use entity::user_roles;
    let role = if let Some(r) = roles::Entity::find()
        .filter(roles::Column::Name.eq("Admin"))
        .one(conn)
        .await
        .unwrap()
    {
        r
    } else {
        let r = roles::ActiveModel {
            id: Set(Uuid::now_v7()),
            name: Set("Admin".to_string()),
        };
        r.insert(conn).await.unwrap()
    };

    user_roles::ActiveModel {
        user_id: Set(user_id),
        role_id: Set(role.id),
    }
    .insert(conn)
    .await
    .unwrap();

    // Create Session
    let session_token = Uuid::now_v7().to_string();
    let session = sessions::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user_id),
        token: Set(session_token.clone()),
        expires_at: Set((chrono::Utc::now() + chrono::Duration::hours(24)).into()),
        created_at: Set(chrono::Utc::now().into()),
        ip_address: Set(Some("127.0.0.1".to_string())),
        user_agent: Set(Some("TestAgent".to_string())),
    };
    session.insert(conn).await.unwrap();

    session_token
}

#[tokio::test]
async fn test_create_user_success() {
    let TestApp { app, conn, .. } = setup().await;
    let token = create_admin_session(&conn).await;

    // Need a role to assign
    let role = roles::ActiveModel {
        id: Set(Uuid::now_v7()),
        name: Set("Editor".to_string()),
    };
    let role = role.insert(&conn).await.unwrap();

    // 1. GET User Create Form to get CSRF Cookie and Token
    let form_response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/admin/users/create")
                .method("GET")
                .header("Cookie", format!("admin_session={}", token))
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let cookie_headers: Vec<String> = form_response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect();

    let body_bytes = axum::body::to_bytes(form_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    // Regex to find token in URL: hx-post="/admin/users?csrf_token=TOKEN"
    let re = regex::Regex::new(r#"csrf_token=([^"&]+)"#).unwrap();
    let csrf_token = re
        .captures(&body_str)
        .expect("CSRF token not found in form URL")
        .get(1)
        .unwrap()
        .as_str()
        .to_string();

    // Combine cookies
    let mut request_cookies = vec![format!("admin_session={}", token)];
    for c in cookie_headers {
        let parts: Vec<&str> = c.split(';').collect();
        if let Some(part) = parts.first() {
            request_cookies.push(part.to_string());
        }
    }
    let full_cookie = request_cookies.join("; ");

    let body = Body::from(format!(
        "name=NewUser&user=newuser&email=new@example.com&password=password123&status=Active&role_ids={}",
        role.id
    ));

    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/admin/users")
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Cookie", full_cookie.clone())
                .header("X-CSRF-TOKEN", csrf_token.clone())
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .body(body)
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success() || response.status().is_redirection());
}

#[tokio::test]
async fn test_create_user_validation_failure() {
    let TestApp { app, conn, .. } = setup().await;
    let token = create_admin_session(&conn).await;

    // 1. GET User Create Form
    let form_response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/admin/users/create")
                .method("GET")
                .header("Cookie", format!("admin_session={}", token))
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let cookie_headers: Vec<String> = form_response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect();

    let body_bytes = axum::body::to_bytes(form_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    // Regex to find token in URL: hx-post="/admin/users?csrf_token=TOKEN"
    let re = regex::Regex::new(r#"csrf_token=([^"&]+)"#).unwrap();
    let csrf_token = re
        .captures(&body_str)
        .expect("CSRF token not found in form URL")
        .get(1)
        .unwrap()
        .as_str()
        .to_string();

    let mut request_cookies = vec![format!("admin_session={}", token)];
    for c in cookie_headers {
        let parts: Vec<&str> = c.split(';').collect();
        if let Some(part) = parts.first() {
            request_cookies.push(part.to_string());
        }
    }
    let full_cookie = request_cookies.join("; ");

    // Invalid Email
    let body = Body::from("name=Val&user=val&email=invalid-email&password=123&status=Active");

    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/admin/users")
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Cookie", full_cookie)
                .header("X-CSRF-TOKEN", csrf_token)
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .body(body)
                .unwrap(),
        )
        .await
        .unwrap();

    // Expect 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
