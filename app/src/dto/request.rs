//! DTOs para request/response dos handlers.
//!
//! Este módulo centraliza structs usadas para query params e form data.
//! Validação é feita usando o crate `validator`.

use serde::Deserialize;
use validator::Validate;

// =============================================================================
// User Query Params
// =============================================================================

#[derive(Deserialize)]
pub struct UsersPageQuery {
    pub page: Option<u64>,
}

/// Parâmetros para criação de usuário.
///
/// Validações:
/// - `name`: 2-100 caracteres
/// - `user`: 3-50 caracteres, apenas alfanuméricos e _
/// - `email`: formato de email válido
/// - `password`: mínimo 8 caracteres
/// - `status`: Active ou Inactive
#[derive(Deserialize, Debug, Validate)]
pub struct CreateUserParams {
    #[validate(length(min = 2, max = 100, message = "Nome deve ter entre 2 e 100 caracteres"))]
    pub name: String,

    #[validate(length(
        min = 3,
        max = 50,
        message = "Usuário deve ter entre 3 e 50 caracteres"
    ))]
    #[validate(regex(
        path = *USERNAME_REGEX,
        message = "Usuário deve conter apenas letras minúsculas, opcionalmente com um ponto (ex: joao.silva)"
    ))]
    pub user: String,

    #[validate(email(message = "Email inválido"))]
    pub email: String,

    #[validate(length(min = 8, message = "Senha deve ter no mínimo 8 caracteres"))]
    pub password: String,

    #[validate(custom(function = "validate_status"))]
    pub status: String,

    /// IDs das roles (ex: role_ids=uuid1&role_ids=uuid2)
    #[serde(default)]
    pub role_ids: Vec<String>,
}

/// Parâmetros para atualização de usuário.
#[derive(Deserialize, Debug, Validate)]
pub struct UserUpdateParams {
    #[validate(length(min = 2, max = 100, message = "Nome deve ter entre 2 e 100 caracteres"))]
    pub name: String,

    #[validate(length(
        min = 3,
        max = 50,
        message = "Usuário deve ter entre 3 e 50 caracteres"
    ))]
    #[validate(regex(
        path = *USERNAME_REGEX,
        message = "Usuário deve conter apenas letras minúsculas, opcionalmente com um ponto (ex: joao.silva)"
    ))]
    pub user: String,

    #[validate(email(message = "Email inválido"))]
    pub email: String,

    /// Senha opcional - validada manualmente no handler se fornecida
    pub password: Option<String>,

    #[validate(custom(function = "validate_status"))]
    pub status: String,

    /// IDs das roles
    #[serde(default)]
    pub role_ids: Vec<String>,
}

// =============================================================================
// Validação Customizada
// =============================================================================

use once_cell::sync::Lazy;
use regex::Regex;

/// Regex para username: apenas alfanuméricos e underscore "^[a-z]+(?:\.[a-z]+)?$" ou somente com ponto "^[a-z]+\.[a-z]+$"
static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z]+(?:\.[a-z]+)?$").unwrap());

/// Valida que o status é Active ou Inactive
fn validate_status(status: &str) -> Result<(), validator::ValidationError> {
    match status {
        "Active" | "Inactive" => Ok(()),
        _ => {
            let mut err = validator::ValidationError::new("invalid_status");
            err.message = Some("Status deve ser 'Active' ou 'Inactive'".into());
            Err(err)
        }
    }
}

// =============================================================================
// RBAC Query Params
// =============================================================================

#[derive(Deserialize)]
pub struct RbacParams {
    pub role: Option<String>,
}

#[derive(Deserialize)]
pub struct RbacToggleParams {
    pub role: String,
    pub resource: String,
    pub action: String,
    pub current_status: String,
}

// =============================================================================
// Audit Logs Query Params
// =============================================================================

#[derive(Deserialize)]
pub struct AuditLogsQuery {
    pub page: Option<u64>,
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub username: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

// =============================================================================
// Post DTOs
// =============================================================================

/// Parâmetros para criação de post
#[derive(Deserialize, Debug, Validate)]
pub struct CreatePostParams {
    #[validate(length(
        min = 5,
        max = 200,
        message = "Título deve ter entre 5 e 200 caracteres"
    ))]
    pub title: String,

    #[validate(length(min = 10, message = "Conteúdo deve ter no mínimo 10 caracteres"))]
    pub text: String,

    pub author: String,

    pub category: String,

    pub status: String,

    #[serde(default)]
    pub image_url: String,
}

/// Parâmetros para atualização de post
#[derive(Deserialize, Debug, Validate)]
pub struct UpdatePostParams {
    #[validate(length(
        min = 5,
        max = 200,
        message = "Título deve ter entre 5 e 200 caracteres"
    ))]
    pub title: String,

    #[validate(length(min = 10, message = "Conteúdo deve ter no mínimo 10 caracteres"))]
    pub text: String,

    pub author: String,

    pub category: String,

    pub status: String,

    #[serde(default)]
    pub image_url: String,
}
