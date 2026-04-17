use axum_example_app::repository::Mutation;
use axum_example_app::utils::password::{hash_password, verify_password};
use entity::users;
use sea_orm::{ConnectionTrait, Database};

async fn setup_db() -> sea_orm::DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.unwrap();

    // Create users table using raw SQL for SQLite
    let sql = r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            username TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL,
            status TEXT NOT NULL,
            is_banned INTEGER NOT NULL DEFAULT 0,
            avatar_url TEXT,
            last_active TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
    "#;

    db.execute_unprepared(sql).await.unwrap();

    db
}

fn create_test_user(name: &str, email: &str) -> users::Model {
    users::Model {
        id: uuid::Uuid::now_v7(),
        name: name.to_string(),
        username: format!("user_{}", name.to_lowercase()),
        email: email.to_string(),
        password: "test_password123".to_string(),
        status: "Active".to_string(),
        is_banned: false,
        avatar_url: None,
        last_active: None,
        created_at: chrono::Utc::now().fixed_offset(),
        updated_at: chrono::Utc::now().fixed_offset(),
    }
}

// --- Password Hash Tests ---

#[test]
fn test_password_hash_and_verify() {
    let password = "SecurePassword123!";
    let hash = hash_password(password).expect("Should hash password");

    assert!(
        verify_password(password, &hash),
        "Should verify correct password"
    );
    assert!(
        !verify_password("wrong_password", &hash),
        "Should reject wrong password"
    );
}

#[test]
fn test_password_hash_is_unique() {
    let password = "SamePassword";
    let hash1 = hash_password(password).expect("First hash");
    let hash2 = hash_password(password).expect("Second hash");

    // Each hash should be different due to random salt
    assert_ne!(hash1, hash2, "Hashes should be unique due to salt");

    // But both should verify correctly
    assert!(verify_password(password, &hash1));
    assert!(verify_password(password, &hash2));
}

// --- User CRUD Tests ---

#[tokio::test]
async fn test_create_user() {
    let db = setup_db().await;

    let user_data = create_test_user("John Doe", "john@example.com");
    let result = Mutation::create_user(&db, user_data).await;

    assert!(result.is_ok(), "Should create user successfully");

    let created_user = result.unwrap();
    assert_eq!(created_user.name, "John Doe");
    assert_eq!(created_user.email, "john@example.com");
    assert_eq!(created_user.status, "Active");
    assert!(!created_user.is_banned);

    // Password should be hashed, not plaintext
    assert_ne!(created_user.password, "test_password123");
    assert!(
        created_user.password.starts_with("$argon2"),
        "Password should be argon2 hash"
    );
}

#[tokio::test]
async fn test_create_and_find_user() {
    use entity::users::Entity as Users;
    use sea_orm::EntityTrait;

    let db = setup_db().await;

    let user_data = create_test_user("Jane Doe", "jane@example.com");
    let created = Mutation::create_user(&db, user_data).await.unwrap();

    // Find by ID
    let found = Users::find_by_id(created.id).one(&db).await.unwrap();

    assert!(found.is_some());
    let found_user = found.unwrap();
    assert_eq!(found_user.name, "Jane Doe");
    assert_eq!(found_user.email, "jane@example.com");
}

#[tokio::test]
async fn test_update_user() {
    let db = setup_db().await;

    // Create user
    let user_data = create_test_user("Update Test", "update@example.com");
    let created = Mutation::create_user(&db, user_data).await.unwrap();

    // Update user
    let update_data = users::Model {
        id: created.id,
        name: "Updated Name".to_string(),
        username: "updated_user".to_string(),
        email: "updated@example.com".to_string(),
        password: "".to_string(), // Empty = don't update password
        status: "Inactive".to_string(),
        is_banned: true,
        avatar_url: None,
        last_active: None,
        created_at: created.created_at,
        updated_at: chrono::Utc::now().fixed_offset(),
    };

    let updated = Mutation::update_user_by_id(&db, created.id, update_data)
        .await
        .unwrap();

    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.email, "updated@example.com");
    assert_eq!(updated.status, "Inactive");
    assert!(updated.is_banned);
}

#[tokio::test]
async fn test_delete_user() {
    use entity::users::Entity as Users;
    use sea_orm::EntityTrait;

    let db = setup_db().await;

    // Create user
    let user_data = create_test_user("Delete Test", "delete@example.com");
    let created = Mutation::create_user(&db, user_data).await.unwrap();

    // Delete user
    let result = Mutation::delete_user(&db, created.id).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().rows_affected, 1);

    // Verify deletion
    let found = Users::find_by_id(created.id).one(&db).await.unwrap();
    assert!(found.is_none(), "User should be deleted");
}

#[tokio::test]
async fn test_delete_nonexistent_user() {
    let db = setup_db().await;

    let fake_id = uuid::Uuid::now_v7();
    let result = Mutation::delete_user(&db, fake_id).await;

    assert!(result.is_err(), "Should fail to delete nonexistent user");
}
