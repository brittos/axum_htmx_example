//! Templates Askama para a área administrativa.
//!
//! Este módulo centraliza todas as structs de template usadas nos handlers admin.

use crate::models::view::{
    AuditLogViewModel, RbacAction, RbacPermission, RbacResource, RbacRole, UserViewModel,
};
use askama::Template;
use std::collections::HashMap;

// =============================================================================
// Dashboard Templates
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Dados do usuário para exibição na sidebar
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SidebarUserInfo {
    pub name: String,
    pub initials: String,
    pub role: String,
    /// Permissões do usuário: resource -> set de actions
    #[serde(default)]
    pub permissions: std::collections::HashMap<String, HashSet<String>>,
}

impl SidebarUserInfo {
    /// Verifica se o usuário tem permissão de leitura para um recurso
    pub fn can_read(&self, resource: &str) -> bool {
        // Admin pode tudo
        if self.role == "Admin" {
            return true;
        }
        self.permissions
            .get(resource)
            .is_some_and(|actions| actions.contains("read"))
    }

    /// Verifica se o usuário tem permissão para uma ação específica
    pub fn has_permission(&self, resource: &str, action: &str) -> bool {
        if self.role == "Admin" {
            return true;
        }
        self.permissions
            .get(resource)
            .is_some_and(|actions| actions.contains(action))
    }
}

#[derive(Template)]
#[template(path = "admin/dashboard_content.html")]
pub struct DashboardContentTemplate {
    pub active_users: u32,
    pub total_posts: u32,
    pub server_load: u32,
    pub system_health: String,
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct DashboardFullTemplate {
    pub title: String,
    pub page_title: String,
    pub active_tab: String,
    pub active_users: u32,
    pub total_posts: u32,
    pub server_load: u32,
    pub system_health: String,
    pub sidebar_user: SidebarUserInfo,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "admin/login.html")]
pub struct LoginTemplate {
    pub title: String,
    pub page_title: String,
    pub active_tab: String,
    pub success_message: Option<String>,
    pub error_message: Option<String>,
}

// =============================================================================
// User Management Templates
// =============================================================================

#[derive(Template)]
#[template(path = "admin/user_management_content.html")]
pub struct UserManagementContentTemplate {
    pub active_tab: String,
    pub users: Option<Vec<UserViewModel>>,
    pub rbac: Option<RbacPartialTemplate>,
    pub page: u64,
    pub total_pages: u64,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "admin/user_management.html")]
pub struct UserManagementFullTemplate {
    pub title: String,
    pub page_title: String,
    pub active_tab: String,
    pub users: Option<Vec<UserViewModel>>,
    pub rbac: Option<RbacPartialTemplate>,
    pub page: u64,
    pub total_pages: u64,
    pub sidebar_user: SidebarUserInfo,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "admin/users_table.html")]
pub struct UsersTableTemplate {
    pub users: Vec<UserViewModel>,
    pub page: u64,
    pub total_pages: u64,
    /// Flags de permissão para controlar visibilidade de botões
    pub can_create: bool,
    pub can_edit: bool,
    pub can_delete: bool,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "admin/user_form_partial.html")]
pub struct UserFormPartialTemplate {
    pub roles: Vec<crate::models::view::RoleViewModel>,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "admin/user_edit_partial.html")]
pub struct UserEditPartialTemplate {
    pub user: entity::users::Model,
    pub roles: Vec<crate::models::view::RoleViewModel>,
    pub current_role_ids: Vec<uuid::Uuid>,
    pub csrf_token: String,
}

// =============================================================================
// RBAC Templates
// =============================================================================

/// Template holder para dados RBAC (não é um Template Askama diretamente)
pub struct RbacPartialTemplate {
    pub active_role: String,
    pub roles: Vec<RbacRole>,
    pub resources: Vec<RbacResource>,
    pub actions: Vec<RbacAction>,
    pub permissions: HashMap<String, HashMap<String, RbacPermission>>,
}

/// Template para botão de toggle de permissão RBAC
#[derive(Template)]
#[template(path = "admin/rbac_toggle_button.html")]
pub struct RbacToggleButtonTemplate {
    pub role: String,
    pub resource: String,
    pub action: String,
    pub status: String,
    pub is_granted: bool,
    pub csrf_token: String,
}

// =============================================================================
// Settings Templates
// =============================================================================

#[derive(Template)]
#[template(path = "admin/settings_content.html")]
pub struct SettingsContentTemplate {}

#[derive(Template)]
#[template(path = "admin/settings.html")]
pub struct SettingsFullTemplate {
    pub title: String,
    pub page_title: String,
    pub active_tab: String,
    pub sidebar_user: SidebarUserInfo,
}

// =============================================================================
// Audit Logs Templates
// =============================================================================

/// Struct para representar usuário no dropdown de filtros
#[derive(Clone)]
pub struct AuditUserOption {
    pub id: String,
    pub name: String,
}

#[derive(Template)]
#[template(path = "admin/audit_logs_partial.html")]
pub struct AuditLogsPartialTemplate {
    pub logs: Vec<AuditLogViewModel>,
    pub current_page: u64,
    pub total_pages: u64,
    pub filter_action: Option<String>,
    pub filter_entity_type: Option<String>,
    pub filter_username: Option<String>,
    pub filter_date_from: Option<String>,
    pub filter_date_to: Option<String>,
    pub users: Vec<crate::handlers::admin::audit::UserOption>,
}

#[derive(Template)]
#[template(path = "admin/audit_logs.html")]
pub struct AuditLogsFullTemplate {
    pub title: String,
    pub page_title: String,
    pub active_tab: String,
    pub logs: Vec<AuditLogViewModel>,
    pub current_page: u64,
    pub total_pages: u64,
    pub filter_action: Option<String>,
    pub filter_entity_type: Option<String>,
    pub filter_username: Option<String>,
    pub filter_date_from: Option<String>,
    pub filter_date_to: Option<String>,
    pub users: Vec<crate::handlers::admin::audit::UserOption>,
    pub sidebar_user: SidebarUserInfo,
}

// =============================================================================
// Posts Templates
// =============================================================================

#[derive(Template)]
#[template(path = "admin/posts_content.html")]
pub struct PostsContentTemplate {
    pub posts: Vec<crate::models::view::PostViewModel>,
    pub page: u64,
    pub total_pages: u64,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "admin/posts.html")]
pub struct PostsFullTemplate {
    pub title: String,
    pub page_title: String,
    pub active_tab: String,
    pub posts: Vec<crate::models::view::PostViewModel>,
    pub page: u64,
    pub total_pages: u64,
    pub sidebar_user: SidebarUserInfo,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "admin/post_form.html")]
pub struct PostFormTemplate {
    pub title: String,
    pub page_title: String,
    pub active_tab: String,
    pub sidebar_user: SidebarUserInfo,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "admin/post_edit_form.html")]
pub struct PostEditFormTemplate {
    pub title: String,
    pub page_title: String,
    pub active_tab: String,
    pub post: entity::post::Model,
    pub sidebar_user: SidebarUserInfo,
    pub csrf_token: String,
}

// =============================================================================
// Profile Templates
// =============================================================================

#[derive(Template)]
#[template(path = "admin/profile_content.html")]
pub struct ProfileContentTemplate {
    pub user: entity::users::Model,
    pub success_message: Option<String>,
    pub error_message: Option<String>,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "admin/profile.html")]
pub struct ProfileFullTemplate {
    pub title: String,
    pub page_title: String,
    pub active_tab: String,
    pub user: entity::users::Model,
    pub success_message: Option<String>,
    pub error_message: Option<String>,
    pub csrf_token: String,
    pub sidebar_user: SidebarUserInfo,
}
