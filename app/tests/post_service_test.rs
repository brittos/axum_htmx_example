use axum_example_app::dto::{CreatePostParams, UpdatePostParams};
use axum_example_app::service::post_service::PostService;
use sea_orm::{EntityTrait, PaginatorTrait};

mod common;

#[tokio::test]
async fn test_create_and_find_post() {
    let app = common::setup().await;
    let conn = &app.conn;

    // 1. Create Post
    let params = CreatePostParams {
        title: "Test Post Title".to_string(),
        text: "This is a test post content with sufficient length.".to_string(),
        author: "Tester".to_string(),
        category: "Testing".to_string(),
        status: "Published".to_string(),
        image_url: "".to_string(),
    };

    let post = PostService::create_post(conn, params)
        .await
        .expect("Failed to create post");

    assert_eq!(post.title, "Test Post Title");

    // 2. Find by ID
    let found = PostService::find_post_by_id(conn, post.id)
        .await
        .expect("Failed to find post")
        .expect("Post not found");

    assert_eq!(found.title, "Test Post Title");
}

#[tokio::test]
async fn test_update_post() {
    let app = common::setup().await;
    let conn = &app.conn;

    // 1. Create Post
    let params = CreatePostParams {
        title: "Original Title".to_string(),
        text: "Original content...".to_string(),
        author: "Author".to_string(),
        category: "Cat".to_string(),
        status: "Draft".to_string(),
        image_url: "".to_string(),
    };
    let post = PostService::create_post(conn, params).await.unwrap();

    // 2. Update Post
    let update_params = UpdatePostParams {
        title: "Updated Title".to_string(),
        text: "Updated content...".to_string(),
        author: "Author".to_string(),
        category: "Cat".to_string(),
        status: "Published".to_string(),
        image_url: "http://example.com/image.png".to_string(),
    };

    let updated = PostService::update_post(conn, post.id, update_params)
        .await
        .expect("Failed to update post");

    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.status, "Published");
    assert_eq!(updated.image_url, "http://example.com/image.png");
}

#[tokio::test]
async fn test_delete_post() {
    let app = common::setup().await;
    let conn = &app.conn;

    // 1. Create Post
    let params = CreatePostParams {
        title: "To Delete".to_string(),
        text: "Content...".to_string(),
        author: "Auth".to_string(),
        category: "Cat".to_string(),
        status: "Draft".to_string(),
        image_url: "".to_string(),
    };
    let post = PostService::create_post(conn, params).await.unwrap();

    // 2. Delete
    PostService::delete_post(conn, post.id)
        .await
        .expect("Failed to delete");

    // 3. Verify
    let found = PostService::find_post_by_id(conn, post.id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_find_posts_in_page() {
    let app = common::setup().await;
    let conn = &app.conn;

    // 0. Ensure Clean State
    entity::post::Entity::delete_many()
        .exec(conn)
        .await
        .unwrap();

    // 1. Create 15 Posts
    for i in 1..=15 {
        let params = CreatePostParams {
            title: format!("Post Title {}", i),
            text: "Content with enough length for validation...".to_string(),
            author: "Auth".to_string(),
            category: "Cat".to_string(),
            status: "Published".to_string(),
            image_url: "".to_string(),
        };
        PostService::create_post(conn, params).await.unwrap();
    }

    let count = entity::post::Entity::find().count(conn).await.unwrap();
    println!("DEBUG: Total posts in DB: {}", count);

    // 2. Page 1 (10 items)
    let (posts, pages) = PostService::find_posts_in_page(conn, 1, 10).await.unwrap();
    println!("DEBUG: Page 1 items: {}", posts.len());
    assert_eq!(posts.len(), 10, "Page 1 should have 10 items");
    assert_eq!(pages, 2, "Should have 2 pages");

    // 3. Page 2 (5 items)
    let (posts_p2, _) = PostService::find_posts_in_page(conn, 2, 10).await.unwrap();
    println!("DEBUG: Page 2 items: {}", posts_p2.len());
    assert_eq!(posts_p2.len(), 5, "Page 2 should have 5 items");
}
