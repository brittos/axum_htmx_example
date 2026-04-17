use axum_example_app::repository::{Mutation, Query};
use entity::post;
use sea_orm::{ActiveValue, Database};

#[tokio::test]
async fn crud_post_tests() {
    let db = &Database::connect("sqlite::memory:").await.unwrap();

    // Create table in memory
    db.get_schema_builder()
        .register(post::Entity)
        .apply(db)
        .await
        .unwrap();

    let now = chrono::Utc::now().naive_utc();
    // Use fixed time or similar if needed for exact comparison, but normally set by DB or Service.
    // In our Mutation::create_post logic, we might set it or pass it.
    // Mutation::create_post uses the date passed in form_data.

    // --- Test Create Post A ---
    {
        let post = Mutation::create_post(
            db,
            post::Model {
                id: 0,
                title: "Title A".to_owned(),
                text: "Text A".to_owned(),
                author: "Author A".to_owned(),
                category: "Category A".to_owned(),
                status: "Published".to_owned(),
                date: now,
                views: 10,
                comments: 5,
                image_url: "http://img.a".to_owned(),
            },
        )
        .await
        .unwrap();

        // Check if returned ActiveModel matches input
        assert_eq!(post.title, ActiveValue::Unchanged("Title A".to_owned()));
        assert_eq!(post.text, ActiveValue::Unchanged("Text A".to_owned()));
        assert_eq!(post.author, ActiveValue::Unchanged("Author A".to_owned()));
    }

    // --- Test Create Post B ---
    {
        let post = Mutation::create_post(
            db,
            post::Model {
                id: 0,
                title: "Title B".to_owned(),
                text: "Text B".to_owned(),
                author: "Author B".to_owned(),
                category: "Category B".to_owned(),
                status: "Draft".to_owned(),
                date: now,
                views: 0,
                comments: 0,
                image_url: "http://img.b".to_owned(),
            },
        )
        .await
        .unwrap();

        assert_eq!(post.title, ActiveValue::Unchanged("Title B".to_owned()));
    }

    // --- Test Read Post A ---
    {
        let post = Query::find_post_by_id(db, 1).await.unwrap().unwrap();

        assert_eq!(post.id, 1);
        assert_eq!(post.title, "Title A");
        assert_eq!(post.author, "Author A");
    }

    // --- Test Update Post A ---
    {
        let post = Mutation::update_post_by_id(
            db,
            1,
            post::Model {
                id: 1,
                title: "New Title A".to_owned(),
                text: "New Text A".to_owned(),
                author: "New Author A".to_owned(),
                category: "New Cat A".to_owned(),
                status: "Updated".to_owned(),
                date: now,
                views: 20,
                comments: 10,
                image_url: "http://img.a.new".to_owned(),
            },
        )
        .await
        .unwrap();

        assert_eq!(post.title, "New Title A");
        assert_eq!(post.text, "New Text A");
        assert_eq!(post.views, 20);
    }

    // --- Test Delete Post B (id: 2) ---
    {
        let result = Mutation::delete_post(db, 2).await.unwrap();

        assert_eq!(result.rows_affected, 1);
    }

    // --- Verify Deletion ---
    {
        let post = Query::find_post_by_id(db, 2).await.unwrap();
        assert!(post.is_none());
    }

    // --- Delete All (remains Post A) ---
    {
        let result = Mutation::delete_all_posts(db).await.unwrap();

        assert_eq!(result.rows_affected, 1);
    }
}
