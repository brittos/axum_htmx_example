//! RBAC Integration Tests
//!
//! Tests the Role-Based Access Control handler functionality.

use axum::{body::Body, http::StatusCode};

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

mod common;
use axum_example_app::service::rbac_service::invalidate_permission_cache;
use common::{login_admin, login_user_with_role, request, request_with_cookies, spawn_app_full};

/// Test that RBAC partial requires authentication
#[tokio::test]
async fn test_rbac_partial_requires_auth() {
    let app_data = spawn_app_full().await;

    let response = request(&app_data.app, "/admin/rbac/partial", "GET", Body::empty()).await;

    // Should redirect to login
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

/// Test that ADMIN user can access RBAC partial
#[tokio::test]
async fn test_rbac_partial_access_admin() {
    let app_data = spawn_app_full().await;

    let cookies = login_admin(&app_data.app, &app_data.conn, "admin_user", "password123").await;

    let response = request_with_cookies(
        &app_data.app,
        "/admin/rbac/partial",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    // Should return 200 OK with partial HTML
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test RBAC partial access denied for user without permission
#[tokio::test]
async fn test_rbac_partial_access_denied() {
    let app_data = spawn_app_full().await;

    // User with "Viewer" role (assumed no edit permissions on User Management)
    let cookies = login_user_with_role(
        &app_data.app,
        &app_data.conn,
        "viewer_user",
        "password123",
        "Viewer",
    )
    .await;

    let response = request_with_cookies(
        &app_data.app,
        "/admin/rbac/partial",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    // Should return 403 Forbidden
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// Test RBAC partial with role filter (Admin)
#[tokio::test]
async fn test_rbac_partial_with_role_filter() {
    let app_data = spawn_app_full().await;

    let cookies = login_admin(
        &app_data.app,
        &app_data.conn,
        "rbac_filter_user",
        "password123",
    )
    .await;

    let response = request_with_cookies(
        &app_data.app,
        "/admin/rbac/partial?role=admin",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

/// Test that RBAC toggle requires authentication
#[tokio::test]
async fn test_rbac_toggle_requires_auth() {
    let app_data = spawn_app_full().await;

    let body = Body::from("role=admin&resource=users&action=create&current_status=denied");

    // Use PATCH method
    let response =
        common::request_with_cookies(&app_data.app, "/admin/rbac/toggle", "PATCH", body, "").await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// Test RBAC Permission Caching
///
/// 1. Admin accesses resource -> Cache Miss -> Cache Set
/// 2. Remove permission from DB manually
/// 3. Access resource again -> Cache Hit (Access Allowed despite DB change)
/// 4. Invalidate Cache
/// 5. Access resource again -> Cache Miss -> DB Check -> Access Denied (Correct)
#[tokio::test]
async fn test_rbac_permission_caching() {
    use entity::{
        actions, permission_actions, permissions, resources, role_permissions, roles, users,
    };
    use redis::AsyncCommands;
    use sea_orm::{ActiveModelTrait, DeleteResult, Set};

    let app_data = spawn_app_full().await;
    let mut redis_conn = app_data.redis.clone(); // Clone redis connection manager

    // 1. Create a user with a specific role and permission
    let cookies = login_user_with_role(
        &app_data.app,
        &app_data.conn,
        "cache_test_user",
        "pass",
        "Editor",
    )
    .await;

    // 1.1 Find the user and role
    let user = users::Entity::find()
        .filter(users::Column::Username.eq("cache_test_user"))
        .one(&app_data.conn)
        .await
        .unwrap()
        .unwrap();

    let role = roles::Entity::find()
        .filter(roles::Column::Name.eq("Editor"))
        .one(&app_data.conn)
        .await
        .unwrap()
        .unwrap();

    // 1.2 Create/Find Resource "User Management"
    let resource = if let Some(r) = resources::Entity::find()
        .filter(resources::Column::Name.eq("User Management"))
        .one(&app_data.conn)
        .await
        .unwrap()
    {
        r
    } else {
        let r = resources::ActiveModel {
            id: Set(uuid::Uuid::now_v7()),
            name: Set("User Management".to_string()),
        };
        r.insert(&app_data.conn).await.expect("Failed resource")
    };

    // 1.3 Create/Find Action "edit"
    let action = if let Some(a) = actions::Entity::find()
        .filter(actions::Column::Name.eq("edit"))
        .one(&app_data.conn)
        .await
        .unwrap()
    {
        a
    } else {
        let a = actions::ActiveModel {
            id: Set(uuid::Uuid::now_v7()),
            name: Set("edit".to_string()),
        };
        a.insert(&app_data.conn).await.expect("Failed action")
    };

    // 1.4 Create/Find Permission (Resource)
    let perm = if let Some(p) = permissions::Entity::find()
        .filter(permissions::Column::ResourceId.eq(resource.id))
        .one(&app_data.conn)
        .await
        .unwrap()
    {
        p
    } else {
        let p = permissions::ActiveModel {
            id: Set(uuid::Uuid::now_v7()),
            resource_id: Set(resource.id),
        };
        p.insert(&app_data.conn).await.expect("Failed permission")
    };

    // 1.5 Link Permission -> Action
    let _ = permission_actions::ActiveModel {
        permission_id: Set(perm.id),
        action_id: Set(action.id),
        ..Default::default()
    }
    .insert(&app_data.conn)
    .await;

    // 1.6 Link Role -> Permission
    let rp = role_permissions::ActiveModel {
        role_id: Set(role.id),
        permission_id: Set(perm.id),
    };
    let _ = rp.insert(&app_data.conn).await;

    // 2. Access Resource (First Time - Cache Miss -> DB Hit -> 200 OK -> Cache Set)
    let response = request_with_cookies(
        &app_data.app,
        "/admin/rbac/partial",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "First access should be allowed"
    );

    // 3. Remove Permission from DB DIRECTLY (RolePermission link)
    let _res: DeleteResult = role_permissions::Entity::delete_many()
        .filter(role_permissions::Column::RoleId.eq(role.id))
        .filter(role_permissions::Column::PermissionId.eq(perm.id))
        .exec(&app_data.conn)
        .await
        .unwrap();

    // 4. Access Resource AGAIN (Should hit Cache and ALLOW)
    let response_cached = request_with_cookies(
        &app_data.app,
        "/admin/rbac/partial",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(
        response_cached.status(),
        StatusCode::OK,
        "Cached access should still be allowed despite DB change"
    );

    // 5. Invalidate Cache using Service Logic (Sidebar + Permissions)
    let sidebar_key = format!("user:sidebar:{}", user.id);
    let _: () = redis_conn.set(&sidebar_key, "dummy").await.unwrap();

    // Call service to invalidate both
    invalidate_permission_cache(&mut redis_conn, user.id).await;

    // Verify sidebar is gone
    let s_exists: bool = redis_conn.exists(&sidebar_key).await.unwrap();
    assert!(!s_exists, "Sidebar cache should be gone");

    // 6. Access Resource AGAIN (Should Miss Cache, hit DB, and DENY)
    let response_denied = request_with_cookies(
        &app_data.app,
        "/admin/rbac/partial",
        "GET",
        Body::empty(),
        &cookies,
    )
    .await;

    assert_eq!(
        response_denied.status(),
        StatusCode::FORBIDDEN,
        "Access should be denied after cache invalidation"
    );
}
