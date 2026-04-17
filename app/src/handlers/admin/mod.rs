//! Handlers do módulo administrativo.
//!
//! Este módulo contém os handlers organizados por domínio.

pub mod audit;
pub mod auth;
pub mod dashboard;
pub mod notifications;
pub mod posts;
pub mod profile;
pub mod rbac;
pub mod settings;
pub mod toasts;
pub mod users;

// Re-exportar todos os handlers públicos
pub use audit::*;
pub use auth::*;
pub use dashboard::*;
pub use notifications::*;
pub use posts::*;
pub use profile::*;
pub use rbac::*;
pub use settings::*;
pub use users::*;
