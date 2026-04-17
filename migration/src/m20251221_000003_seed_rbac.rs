use entity::{
    actions, permission_actions, permissions, resources, role_permissions, roles, user_roles, users,
};
use sea_orm::{ActiveModelTrait, Set};
use sea_orm_migration::prelude::*;
use uuid::Uuid;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. Roles
        let role_names = vec!["Admin", "Sales", "Manager", "Support"];
        let mut role_map = std::collections::HashMap::new();

        for name in role_names {
            let role = roles::ActiveModel {
                id: Set(Uuid::now_v7()),
                name: Set(name.to_owned()),
            }
            .insert(db)
            .await?;
            role_map.insert(name, role);
        }

        let admin_role = role_map.get("Admin").expect("Admin role created");

        // 2. Resources (5 examples)
        let resource_names = vec![
            "Dashboard",
            "User Management",
            "Posts",
            "Audit Logs",
            "Settings",
        ];

        let mut res_models = Vec::new();
        for name in resource_names {
            let res = resources::ActiveModel {
                id: Set(Uuid::now_v7()),
                name: Set(name.to_owned()),
            }
            .insert(db)
            .await?;
            res_models.push(res);
        }

        // 3. Actions
        let action_names = vec!["read", "create", "edit", "delete", "approve"];
        let mut act_models = Vec::new();
        for name in action_names {
            let act = actions::ActiveModel {
                id: Set(Uuid::now_v7()),
                name: Set(name.to_owned()),
            }
            .insert(db)
            .await?;
            act_models.push(act);
        }

        // 4. Permissions (Create one Permission per Resource, assign to Admin, enable all Actions)
        for res in &res_models {
            // Create Permission for this Resource
            let perm = permissions::ActiveModel {
                id: Set(Uuid::now_v7()),
                resource_id: Set(res.id),
            }
            .insert(db)
            .await?;

            // Assign Permission to Admin Role
            role_permissions::ActiveModel {
                role_id: Set(admin_role.id),
                permission_id: Set(perm.id),
            }
            .insert(db)
            .await?;

            // Enable all Actions for this Permission
            for act in &act_models {
                permission_actions::ActiveModel {
                    permission_id: Set(perm.id),
                    action_id: Set(act.id),
                    allowed: Set(true),
                }
                .insert(db)
                .await?;
            }
        }

        // 5. Seed Admin User
        let user = users::ActiveModel {
            id: Set(Uuid::now_v7()),
            name: Set("Bero Buk".to_owned()),
            email: Set("admin@admin.com".to_owned()),
            username: Set("admin".to_owned()),
            password: Set("$argon2id$v=19$m=19456,t=2,p=1$JAOFoKD7mgqgjAVE+QA6WA$eW+dIivWx2uLqL+lVJjFopXyKzy2uKZZxQqpQecobag".to_owned()), // Placeholder hash
            status: Set("Active".to_owned()),
            is_banned: Set(false),
            // Timestamps usually handled by DB default, but entity might require them if not Option
            created_at: Set(chrono::Utc::now().fixed_offset()),
            updated_at: Set(chrono::Utc::now().fixed_offset()),
            ..Default::default()
        }
        .insert(db)
        .await?;

        // 6. Assign Admin Role to User
        user_roles::ActiveModel {
            user_id: Set(user.id),
            role_id: Set(admin_role.id),
        }
        .insert(db)
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            DELETE FROM user_roles;
            DELETE FROM users WHERE email = 'emma@Bero.com';
            DELETE FROM role_permissions;
            DELETE FROM permission_actions;
            DELETE FROM permissions;
            DELETE FROM actions;
            DELETE FROM resources;
            DELETE FROM roles;
        "#;
        crate::exec_stmt(manager, sql).await
    }
}
