use crate::common::{get_random_email, login_user, setup};
use axum_example_app::service::notification_service::{self, NotificationType};
use entity::users;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

mod common;

#[tokio::test]
async fn test_notification_lifecycle_and_security() {
    let app_data = setup().await;
    let app = &app_data.app;
    let conn = &app_data.conn;

    // 1. Criar dois usuários usando o helper login_user (que cria se não existe)
    let user1_name = get_random_email();
    let user2_name = get_random_email();
    let password = "password123";

    login_user(app, conn, &user1_name, password).await;
    login_user(app, conn, &user2_name, password).await;

    // Buscar IDs dos usuários criados
    let user1 = users::Entity::find()
        .filter(users::Column::Username.eq(&user1_name))
        .one(conn)
        .await
        .unwrap()
        .expect("User 1 not found");

    let user2 = users::Entity::find()
        .filter(users::Column::Username.eq(&user2_name))
        .one(conn)
        .await
        .unwrap()
        .expect("User 2 not found");

    // 2. Criar notificação para User 1
    let notif = notification_service::create(
        conn,
        user1.id,
        "Test Notification",
        "Hello User 1",
        NotificationType::Info,
        None,
    )
    .await
    .expect("Failed to create notification");

    // 3. Verificar que User 1 vê a notificação
    let unread_user1 = notification_service::list_unread(conn, user1.id)
        .await
        .expect("Failed to list notifications");
    assert_eq!(
        unread_user1.len(),
        1,
        "User 1 should see 1 unread notification"
    );
    assert_eq!(unread_user1[0].id, notif.id);

    // 4. Verificar que User 2 NÃO vê a notificação (Segurança)
    let unread_user2 = notification_service::list_unread(conn, user2.id)
        .await
        .expect("Failed to list notifications for user 2");
    assert_eq!(
        unread_user2.len(),
        0,
        "User 2 should NOT see user 1's notifications"
    );

    // 5. Testar contador
    let count = notification_service::count_unread(conn, user1.id)
        .await
        .unwrap();
    assert_eq!(count, 1);

    // 6. User 1 marca como lida
    notification_service::mark_as_read(conn, notif.id, user1.id)
        .await
        .expect("Failed to mark as read");

    // Verificar que não é mais listada como unread
    let unread_after = notification_service::list_unread(conn, user1.id)
        .await
        .unwrap();
    assert_eq!(unread_after.len(), 0);

    // 7. Testar IDOR: Tentar marcar como lida com User 2
    // Criar nova notificação para User 1
    let notif2 = notification_service::create(
        conn,
        user1.id,
        "Another One",
        "Security Check",
        NotificationType::Warning,
        None,
    )
    .await
    .unwrap();

    // User 2 tenta marcar notif2 (que é do User 1) como lida
    notification_service::mark_as_read(conn, notif2.id, user2.id)
        .await
        .expect("Should not fail, but should not have effect");

    // Verificar que notif2 AINDA está unread para User 1
    let count_after_attempt = notification_service::count_unread(conn, user1.id)
        .await
        .unwrap();
    assert_eq!(
        count_after_attempt, 1,
        "Notification should still be unread because User 2 cannot mark it"
    );
}

#[tokio::test]
async fn test_cleanup_old_notifications() {
    let app_data = setup().await;
    let app = &app_data.app;
    let conn = &app_data.conn;
    let user_name = get_random_email();
    login_user(app, conn, &user_name, "pass").await;

    let user = users::Entity::find()
        .filter(users::Column::Username.eq(&user_name))
        .one(conn)
        .await
        .unwrap()
        .unwrap();

    // Criar notificação antiga e LIDA
    let notif = notification_service::create(
        conn,
        user.id,
        "Old",
        "Old Msg",
        NotificationType::Info,
        None,
    )
    .await
    .unwrap();

    // Manipular data para ser antiga (31 dias atrás)
    use entity::notifications;
    use sea_orm::{ActiveModelTrait, Set};

    let mut active: notifications::ActiveModel = notif.clone().into();
    active.is_read = Set(true);
    let old_date = chrono::Utc::now().fixed_offset() - chrono::Duration::days(31);
    active.created_at = Set(old_date);
    active.update(conn).await.unwrap();

    // Executar cleanup (older than 30 days)
    let deleted = notification_service::cleanup_old_notifications(conn, 30)
        .await
        .unwrap();

    assert_eq!(deleted, 1, "Should delete 1 old read notification");

    // Verificar que não existe mais
    let check = notifications::Entity::find_by_id(notif.id)
        .one(conn)
        .await
        .unwrap();
    assert!(check.is_none());
}
