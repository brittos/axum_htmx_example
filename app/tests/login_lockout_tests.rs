use crate::common::{get_random_email, setup};
use axum_example_app::service::login_service;
use sea_orm::ColumnTrait;
use sea_orm::{EntityTrait, QueryFilter};

mod common;

#[tokio::test]
async fn test_login_lockout_mechanism() {
    let app_data = setup().await;
    let conn = &app_data.conn;
    let username = get_random_email();
    let ip = "127.0.0.1".to_string();

    // Configuração do teste
    let max_attempts = 3;
    let lockout_minutes = 15;

    // 1. Simular 3 tentativas falhas
    for i in 1..=max_attempts {
        let result = login_service::can_attempt_login(conn, &username, &ip, max_attempts)
            .await
            .expect("Failed to check login attempt");

        assert!(
            matches!(result, login_service::LoginAttemptResult::Allowed),
            "Attempt {} should be allowed",
            i
        );

        // Registrar falha
        login_service::record_failed_attempt(conn, &username, &ip, max_attempts, lockout_minutes)
            .await
            .expect("Failed to record attempt");
    }

    // 2. A próxima tentativa deve ser bloqueada
    let result = login_service::can_attempt_login(conn, &username, &ip, max_attempts)
        .await
        .expect("Failed to check login attempt");

    match result {
        login_service::LoginAttemptResult::Blocked { minutes_remaining } => {
            assert!(minutes_remaining > 0);
            assert!(minutes_remaining <= lockout_minutes as i64);
        }
        _ => panic!(
            "User should be blocked after {} failed attempts",
            max_attempts
        ),
    }

    // 3. Simular expiração do bloqueio via manipulação direta do banco
    use entity::login_attempts;
    use sea_orm::{ActiveModelTrait, Set};

    let attempt = login_attempts::Entity::find()
        .filter(login_attempts::Column::Username.eq(&username))
        .filter(login_attempts::Column::IpAddress.eq(&ip))
        .one(conn)
        .await
        .expect("Query failed")
        .expect("Attempt record not found");

    let mut active: login_attempts::ActiveModel = attempt.into();
    // Voltar no tempo (lockout + 1 minute)
    let past_time =
        chrono::Utc::now().fixed_offset() - chrono::Duration::minutes((lockout_minutes + 1).into());
    active.locked_until = Set(Some(past_time));
    active
        .update(conn)
        .await
        .expect("Failed to update timestamp");

    // 4. Tentar novamente - deve ser permitido agora
    let result_after_wait = login_service::can_attempt_login(conn, &username, &ip, max_attempts)
        .await
        .expect("Failed to check login attempt");

    assert!(
        matches!(
            result_after_wait,
            login_service::LoginAttemptResult::Allowed
        ),
        "User should be unblocked after time passes"
    );
}

#[tokio::test]
async fn test_login_success_clears_attempts() {
    let app_data = setup().await;
    let conn = &app_data.conn;
    let username = get_random_email();
    let ip = "192.168.1.50".to_string();
    let max_attempts = 5;

    // Registrar algumas falhas
    login_service::record_failed_attempt(conn, &username, &ip, max_attempts, 10)
        .await
        .unwrap();
    login_service::record_failed_attempt(conn, &username, &ip, max_attempts, 10)
        .await
        .unwrap();

    // Verificar que existem falhas
    use entity::login_attempts;
    let attempt = login_attempts::Entity::find()
        .filter(login_attempts::Column::Username.eq(&username))
        .one(conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(attempt.attempts, 2);

    // Simular Login Com Sucesso -> Limpar tentativas
    login_service::clear_attempts(conn, &username, &ip)
        .await
        .unwrap();

    // Verificar que registro foi removido
    let attempt_after = login_attempts::Entity::find()
        .filter(login_attempts::Column::Username.eq(&username))
        .one(conn)
        .await
        .unwrap();

    assert!(
        attempt_after.is_none(),
        "Login attempts record should be deleted after success"
    );
}
