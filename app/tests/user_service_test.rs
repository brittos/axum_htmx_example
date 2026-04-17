use axum_example_app::dto::{CreateUserParams, UserUpdateParams};
use axum_example_app::service::user_service::UserService;
use sea_orm::EntityTrait;

mod common;

#[tokio::test]
async fn test_create_user_with_roles() {
    let app = common::setup().await;
    let conn = &app.conn;

    // 1. Setup Roles
    use entity::roles;
    use sea_orm::{ActiveModelTrait, Set};

    let role1 = roles::ActiveModel {
        id: Set(uuid::Uuid::now_v7()),
        name: Set("Editor".to_string()),
    };
    let role1 = role1.insert(conn).await.expect("Failed to create role 1");

    let role2 = roles::ActiveModel {
        id: Set(uuid::Uuid::now_v7()),
        name: Set("Viewer".to_string()),
    };
    let role2 = role2.insert(conn).await.expect("Failed to create role 2");

    // 2. Create User via Service
    let params = CreateUserParams {
        name: "Test User".to_string(),
        user: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
        status: "Active".to_string(),
        role_ids: vec![role1.id.to_string(), role2.id.to_string()],
    };

    let created_user = UserService::create_user_with_roles(conn, params)
        .await
        .expect("Failed to create user with roles");

    assert_eq!(created_user.username, "testuser");

    // 3. Verify Roles
    let user_with_roles = UserService::find_user_with_roles(conn, created_user.id)
        .await
        .expect("Failed to query user")
        .expect("User not found");

    assert_eq!(user_with_roles.role_names.len(), 2);
    assert!(user_with_roles.role_names.contains(&"Editor".to_string()));
    assert!(user_with_roles.role_names.contains(&"Viewer".to_string()));
}

#[tokio::test]
async fn test_update_user_roles_transaction() {
    let app = common::setup().await;
    let conn = &app.conn;

    // 1. Create Roles
    use entity::roles;
    use sea_orm::{ActiveModelTrait, Set};
    let role_a = roles::ActiveModel {
        id: Set(uuid::Uuid::now_v7()),
        name: Set("Role A".to_string()),
    }
    .insert(conn)
    .await
    .unwrap();

    let role_b = roles::ActiveModel {
        id: Set(uuid::Uuid::now_v7()),
        name: Set("Role B".to_string()),
    }
    .insert(conn)
    .await
    .unwrap();

    // 2. Create Initial User
    let params = CreateUserParams {
        name: "Updater".to_string(),
        user: "updater".to_string(),
        email: "updater@example.com".to_string(),
        password: "password".to_string(),
        status: "Active".to_string(),
        role_ids: vec![role_a.id.to_string()],
    };
    let user = UserService::create_user_with_roles(conn, params)
        .await
        .unwrap();

    // 3. Update User: Change Roles from [A] to [B]
    let update_params = UserUpdateParams {
        name: "Updater Modified".to_string(),
        user: "updater".to_string(),
        email: "updater@example.com".to_string(),
        role_ids: vec![role_b.id.to_string()],
        status: "Active".to_string(),
        password: None,
    };

    let updated = UserService::update_user_with_roles(conn, user.id, update_params)
        .await
        .expect("Failed to update user");

    assert_eq!(updated.name, "Updater Modified");

    // 4. Verify Roles
    let user_with_roles = UserService::find_user_with_roles(conn, user.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user_with_roles.role_names.len(), 1);
    assert!(user_with_roles.role_names.contains(&"Role B".to_string()));
    assert!(!user_with_roles.role_names.contains(&"Role A".to_string()));
}

#[tokio::test]
async fn test_delete_user_cascades_permissions() {
    let app = common::setup().await;
    let conn = &app.conn;

    // 0. Create Role (Required for user creation validation)
    use entity::roles;
    use sea_orm::{ActiveModelTrait, Set};
    let role = roles::ActiveModel {
        id: Set(uuid::Uuid::now_v7()),
        name: Set("DeletableRole".to_string()),
    }
    .insert(conn)
    .await
    .unwrap();

    // 1. Create User
    let params = CreateUserParams {
        name: "Deletable".to_string(),
        user: "deletee".to_string(),
        email: "del@example.com".to_string(),
        password: "password".to_string(),
        status: "Active".to_string(),
        role_ids: vec![role.id.to_string()],
    };
    let user = UserService::create_user_with_roles(conn, params)
        .await
        .unwrap();

    // 2. Delete User
    UserService::delete_user_with_roles(conn, user.id)
        .await
        .expect("Failed to delete");

    // 3. Verify Gone
    let found = entity::users::Entity::find_by_id(user.id)
        .one(conn)
        .await
        .unwrap();
    assert!(found.is_none());
}
