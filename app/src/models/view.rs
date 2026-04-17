use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct PostViewModel {
    pub id: u32,
    pub title: String,
    pub author: String,
    pub category: String,
    pub status: String,
    pub date: String,
    pub views: String,
    pub comments: u32,
    pub image_url: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RoleBadge {
    pub name: String,
    pub color: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RoleViewModel {
    pub id: uuid::Uuid,
    pub name: String,
    pub color: String,
}

impl From<entity::roles::Model> for RoleViewModel {
    fn from(model: entity::roles::Model) -> Self {
        Self {
            id: model.id,
            color: crate::config::ui::role_color(&model.name),
            name: model.name,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserViewModel {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
    pub roles: Vec<RoleBadge>,
    pub status: String,
    pub avatar_initials: String,
}

impl From<entity::users::Model> for UserViewModel {
    fn from(u: entity::users::Model) -> Self {
        Self {
            id: u.id,
            name: u.name,
            email: u.email,
            roles: vec![RoleBadge {
                name: "Sem Role".to_string(),
                color: crate::config::ui::role_color("Sem Role"),
            }],
            status: u.status,
            avatar_initials: u
                .username
                .chars()
                .take(2)
                .collect::<String>()
                .to_uppercase(),
        }
    }
}

impl From<crate::service::user_service::UserWithRole> for UserViewModel {
    fn from(uwr: crate::service::user_service::UserWithRole) -> Self {
        let u = uwr.user;
        Self {
            id: u.id,
            name: u.name,
            email: u.email,
            roles: uwr
                .role_names
                .into_iter()
                .map(|name| RoleBadge {
                    color: crate::config::ui::role_color(&name),
                    name,
                })
                .collect(),
            status: u.status,
            avatar_initials: u
                .username
                .chars()
                .take(2)
                .collect::<String>()
                .to_uppercase(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RbacRole {
    pub name: String,
    pub bg_color: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RbacResource {
    pub name: String,
    pub icon: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RbacAction {
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RbacPermission {
    pub actions: std::collections::HashMap<String, bool>,
}

/// View model para exibição de audit logs
#[derive(Clone, Serialize, Deserialize)]
pub struct AuditLogViewModel {
    pub id: String,
    pub action: String,
    pub action_badge_class: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
    pub created_at_relative: String,
}

impl From<entity::audit_logs::Model> for AuditLogViewModel {
    fn from(model: entity::audit_logs::Model) -> Self {
        let action_badge_class = match model.action.as_str() {
            "create" => "badge--success",
            "update" => "badge--info",
            "delete" => "badge--danger",
            "login" => "badge--purple",
            "logout" => "badge--secondary",
            _ => "badge--default",
        }
        .to_string();

        // Formatar data: "2024-12-24 15:54:28" (ISO format)
        let dt = model.created_at;
        let formatted_date = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second()
        );

        // Calcular tempo relativo
        let now = chrono::Utc::now().fixed_offset();
        let diff = now.signed_duration_since(dt);
        let relative = if diff.num_seconds() < 60 {
            "Agora".to_string()
        } else if diff.num_minutes() < 60 {
            format!("Há {} min", diff.num_minutes())
        } else if diff.num_hours() < 24 {
            format!("Há {} h", diff.num_hours())
        } else if diff.num_days() < 7 {
            format!("Há {} dias", diff.num_days())
        } else {
            format!("Há {} sem", diff.num_weeks())
        };

        Self {
            id: model.id.to_string(),
            action: model.action,
            action_badge_class,
            entity_type: model.entity_type,
            entity_id: model.entity_id.map(|id| id.to_string()),
            user_id: model.user_id.map(|id| id.to_string()),
            username: None, // Preenchido pelo handler
            details: model.details,
            ip_address: model.ip_address,
            created_at: formatted_date,
            created_at_relative: relative,
        }
    }
}
