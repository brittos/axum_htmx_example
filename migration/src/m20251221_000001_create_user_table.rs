use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop existing table if it exists
        manager
            .drop_table(Table::drop().table(User::Table).if_exists().to_owned())
            .await?;

        // Recreate table with new schema
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(uuid(User::Id).primary_key().default(Expr::cust("uuidv7()")))
                    .col(string(User::Name))
                    .col(string(User::Username))
                    .col(string(User::Email).unique_key())
                    .col(string(User::Password))
                    .col(string(User::Status).default("Active"))
                    .col(boolean(User::IsBanned).default(false))
                    .col(string(User::AvatarUrl).null()) // Field might be nullable, explicit nullability is better
                    .col(timestamp(User::LastActive).null())
                    .col(
                        timestamp_with_time_zone(User::CreatedAt)
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        timestamp_with_time_zone(User::UpdatedAt)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).if_exists().to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum User {
    #[sea_orm(iden = "users")]
    Table,
    Id,
    Name,
    Username,
    Email,
    Password,
    Status,
    IsBanned,
    AvatarUrl,
    LastActive,
    CreatedAt,
    UpdatedAt,
}
