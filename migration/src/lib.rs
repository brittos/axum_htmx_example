pub use sea_orm_migration::prelude::*;

mod m20220120_000001_create_post_table;
mod m20220120_000002_seed_posts;
mod m20251221_000001_create_user_table;
mod m20251221_000002_create_rbac_tables;
mod m20251221_000003_seed_rbac;
mod m20251222_000001_create_audit_logs;
mod m20251225_000001_create_sessions;
mod m20251225_000002_create_password_resets;
mod m20251230_000001_create_login_attempts;
mod m20251230_000002_create_notifications;

// Migrator struct to handle migrations
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220120_000001_create_post_table::Migration),
            Box::new(m20220120_000002_seed_posts::Migration),
            Box::new(m20251221_000001_create_user_table::Migration),
            Box::new(m20251221_000002_create_rbac_tables::Migration),
            Box::new(m20251221_000003_seed_rbac::Migration),
            Box::new(m20251222_000001_create_audit_logs::Migration),
            Box::new(m20251225_000001_create_sessions::Migration),
            Box::new(m20251225_000002_create_password_resets::Migration),
            Box::new(m20251230_000001_create_login_attempts::Migration),
            Box::new(m20251230_000002_create_notifications::Migration),
        ]
    }
}

pub async fn exec_stmt(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute_unprepared(sql)
        .await
        .map(|_| ())
}
