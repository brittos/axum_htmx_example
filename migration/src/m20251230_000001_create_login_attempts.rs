//! Migração para criar tabela de tentativas de login.
//!
//! Esta tabela armazena tentativas de login falhas para implementar
//! bloqueio temporário após múltiplas tentativas.
//! NÃO tem FK para users - precisa logar tentativas de usernames inexistentes.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LoginAttempts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LoginAttempts::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LoginAttempts::Username)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LoginAttempts::IpAddress)
                            .string_len(45)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LoginAttempts::Attempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(LoginAttempts::LockedUntil)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LoginAttempts::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(LoginAttempts::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Índice para busca por username
        manager
            .create_index(
                Index::create()
                    .name("idx_login_attempts_username")
                    .table(LoginAttempts::Table)
                    .col(LoginAttempts::Username)
                    .to_owned(),
            )
            .await?;

        // Índice para busca por IP
        manager
            .create_index(
                Index::create()
                    .name("idx_login_attempts_ip")
                    .table(LoginAttempts::Table)
                    .col(LoginAttempts::IpAddress)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LoginAttempts::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LoginAttempts {
    Table,
    Id,
    Username,
    IpAddress,
    Attempts,
    LockedUntil,
    CreatedAt,
    UpdatedAt,
}
