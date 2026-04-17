use entity::audit_logs;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

/// Log an audit event
pub struct AuditBuilder<'a> {
    conn: &'a DatabaseConnection,
    action: String,
    entity_type: String,
    entity_id: Option<uuid::Uuid>,
    user_id: Option<uuid::Uuid>,
    ip_address: Option<String>,
    details: Option<String>,
}

impl<'a> AuditBuilder<'a> {
    pub fn new(
        conn: &'a DatabaseConnection,
        action: impl Into<String>,
        entity_type: impl Into<String>,
    ) -> Self {
        Self {
            conn,
            action: action.into(),
            entity_type: entity_type.into(),
            entity_id: None,
            user_id: None,
            ip_address: None,
            details: None,
        }
    }

    pub fn entity_id(mut self, id: uuid::Uuid) -> Self {
        self.entity_id = Some(id);
        self
    }

    pub fn author(mut self, user_id: Option<uuid::Uuid>) -> Self {
        self.user_id = user_id;
        self
    }

    pub fn ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    pub fn details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub async fn log(self) {
        let audit_log = audit_logs::ActiveModel {
            id: Set(uuid::Uuid::now_v7()),
            action: Set(self.action),
            entity_type: Set(self.entity_type),
            entity_id: Set(self.entity_id),
            user_id: Set(self.user_id),
            details: Set(self.details),
            ip_address: Set(self.ip_address),
            created_at: Set(chrono::Utc::now().fixed_offset()),
        };

        if let Err(e) = audit_log.insert(self.conn).await {
            tracing::error!("Failed to log audit event: {}", e);
        }
    }
}

/// Helper function to maintain backward compatibility during strict refactor,
/// but encouraged to use AuditBuilder::new().
pub async fn log_action(
    conn: &DatabaseConnection,
    action: &str,
    entity_type: &str,
    entity_id: Option<uuid::Uuid>,
    user_id: Option<uuid::Uuid>,
    ip_address: Option<String>,
    details: Option<String>,
) {
    let mut builder = AuditBuilder::new(conn, action, entity_type).author(user_id);

    if let Some(id) = entity_id {
        builder = builder.entity_id(id);
    }
    if let Some(ip) = ip_address {
        builder = builder.ip(ip);
    }
    if let Some(d) = details {
        builder = builder.details(d);
    }

    builder.log().await;
}

#[cfg(test)]
mod tests {
    /// Note: log_action is async and requires a real DB connection.
    /// Integration tests for audit logging are in audit_tests.rs.
    /// This module tests any pure helper functions if added in the future.

    #[test]
    fn test_audit_service_module_compiles() {
        // Placeholder test to ensure module compiles correctly
    }
}
