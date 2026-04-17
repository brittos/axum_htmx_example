//! Migração para criar tabela de notificações.
//!
//! Esta tabela armazena notificações in-app para usuários do dashboard.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Notifications::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Notifications::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Notifications::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(Notifications::Title)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Notifications::Message).text().not_null())
                    .col(
                        ColumnDef::new(Notifications::NotificationType)
                            .string_len(20)
                            .not_null()
                            .default("info"),
                    )
                    .col(
                        ColumnDef::new(Notifications::IsRead)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Notifications::ActionUrl)
                            .string_len(500)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Notifications::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notifications_user")
                            .from(Notifications::Table, Notifications::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Índice para busca por user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_notifications_user")
                    .table(Notifications::Table)
                    .col(Notifications::UserId)
                    .to_owned(),
            )
            .await?;

        // Índice parcial para notificações não lidas
        manager
            .create_index(
                Index::create()
                    .name("idx_notifications_unread")
                    .table(Notifications::Table)
                    .col(Notifications::UserId)
                    .col(Notifications::IsRead)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Notifications::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Notifications {
    Table,
    Id,
    UserId,
    Title,
    Message,
    NotificationType,
    IsRead,
    ActionUrl,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
