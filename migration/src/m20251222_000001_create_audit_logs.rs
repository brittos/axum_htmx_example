use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuditLogs::Table)
                    .if_not_exists()
                    .col(
                        uuid(AuditLogs::Id)
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(string(AuditLogs::Action).not_null())
                    .col(string(AuditLogs::EntityType).not_null())
                    .col(uuid_null(AuditLogs::EntityId))
                    .col(uuid_null(AuditLogs::UserId))
                    .col(text_null(AuditLogs::Details))
                    .col(string_null(AuditLogs::IpAddress))
                    .col(
                        timestamp_with_time_zone(AuditLogs::CreatedAt)
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on action column for faster queries
        manager
            .create_index(
                Index::create()
                    .name("idx_audit_logs_action")
                    .table(AuditLogs::Table)
                    .col(AuditLogs::Action)
                    .to_owned(),
            )
            .await?;

        // Create index on entity_type for faster filtering
        manager
            .create_index(
                Index::create()
                    .name("idx_audit_logs_entity_type")
                    .table(AuditLogs::Table)
                    .col(AuditLogs::EntityType)
                    .to_owned(),
            )
            .await?;

        // Create index on created_at for time-based queries
        manager
            .create_index(
                Index::create()
                    .name("idx_audit_logs_created_at")
                    .table(AuditLogs::Table)
                    .col(AuditLogs::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuditLogs::Table).if_exists().to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AuditLogs {
    Table,
    Id,
    Action,
    EntityType,
    EntityId,
    UserId,
    Details,
    IpAddress,
    CreatedAt,
}
