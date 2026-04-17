use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Roles
        manager
            .create_table(
                Table::create()
                    .table(Roles::Table)
                    .if_not_exists()
                    .col(
                        uuid(Roles::Id)
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(string(Roles::Name).unique_key())
                    .to_owned(),
            )
            .await?;

        // User Roles
        manager
            .create_table(
                Table::create()
                    .table(UserRoles::Table)
                    .if_not_exists()
                    .col(uuid(UserRoles::UserId))
                    .col(uuid(UserRoles::RoleId))
                    .primary_key(
                        Index::create()
                            .col(UserRoles::UserId)
                            .col(UserRoles::RoleId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_roles_user")
                            .from(UserRoles::Table, UserRoles::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_roles_role")
                            .from(UserRoles::Table, UserRoles::RoleId)
                            .to(Roles::Table, Roles::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Resources
        manager
            .create_table(
                Table::create()
                    .table(Resources::Table)
                    .if_not_exists()
                    .col(
                        uuid(Resources::Id)
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(string(Resources::Name).unique_key())
                    .to_owned(),
            )
            .await?;

        // Actions
        manager
            .create_table(
                Table::create()
                    .table(Actions::Table)
                    .if_not_exists()
                    .col(
                        uuid(Actions::Id)
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(string(Actions::Name).unique_key())
                    .to_owned(),
            )
            .await?;

        // Permissions
        manager
            .create_table(
                Table::create()
                    .table(Permissions::Table)
                    .if_not_exists()
                    .col(
                        uuid(Permissions::Id)
                            .primary_key()
                            .default(Expr::cust("uuidv7()")),
                    )
                    .col(uuid(Permissions::ResourceId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_permissions_resource")
                            .from(Permissions::Table, Permissions::ResourceId)
                            .to(Resources::Table, Resources::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Permission Actions
        manager
            .create_table(
                Table::create()
                    .table(PermissionActions::Table)
                    .if_not_exists()
                    .col(uuid(PermissionActions::PermissionId))
                    .col(uuid(PermissionActions::ActionId))
                    .col(boolean(PermissionActions::Allowed).default(false))
                    .primary_key(
                        Index::create()
                            .col(PermissionActions::PermissionId)
                            .col(PermissionActions::ActionId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_permission_actions_permission")
                            .from(PermissionActions::Table, PermissionActions::PermissionId)
                            .to(Permissions::Table, Permissions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_permission_actions_action")
                            .from(PermissionActions::Table, PermissionActions::ActionId)
                            .to(Actions::Table, Actions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Role Permissions
        manager
            .create_table(
                Table::create()
                    .table(RolePermissions::Table)
                    .if_not_exists()
                    .col(uuid(RolePermissions::RoleId))
                    .col(uuid(RolePermissions::PermissionId))
                    .primary_key(
                        Index::create()
                            .col(RolePermissions::RoleId)
                            .col(RolePermissions::PermissionId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_role_permissions_role")
                            .from(RolePermissions::Table, RolePermissions::RoleId)
                            .to(Roles::Table, Roles::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_role_permissions_permission")
                            .from(RolePermissions::Table, RolePermissions::PermissionId)
                            .to(Permissions::Table, Permissions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(RolePermissions::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(PermissionActions::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Permissions::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Actions::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Resources::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(UserRoles::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Roles::Table).if_exists().to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum User {
    #[sea_orm(iden = "users")]
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Roles {
    Table,
    Id,
    Name,
}

#[derive(DeriveIden)]
enum UserRoles {
    Table,
    UserId,
    RoleId,
}

#[derive(DeriveIden)]
enum Resources {
    Table,
    Id,
    Name,
}

#[derive(DeriveIden)]
enum Actions {
    Table,
    Id,
    Name,
}

#[derive(DeriveIden)]
enum Permissions {
    Table,
    Id,
    ResourceId,
}

#[derive(DeriveIden)]
enum PermissionActions {
    Table,
    PermissionId,
    ActionId,
    Allowed,
}

#[derive(DeriveIden)]
enum RolePermissions {
    Table,
    RoleId,
    PermissionId,
}
