use ::entity::{audit_logs, audit_logs::Entity as AuditLog};
use sea_orm::*;

pub struct AuditQuery;

#[derive(Debug, Default)]
pub struct AuditLogFilters<'a> {
    pub action: Option<&'a str>,
    pub entity_type: Option<&'a str>,
    pub user_id: Option<uuid::Uuid>,
    pub date_from: Option<chrono::NaiveDate>,
    pub date_to: Option<chrono::NaiveDate>,
}

impl AuditQuery {
    /// Lista paginada de audit logs, ordenados por data decrescente
    pub async fn find_all_paginated(
        db: &DbConn,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<audit_logs::Model>, u64), DbErr> {
        let paginator = AuditLog::find()
            .order_by_desc(audit_logs::Column::CreatedAt)
            .paginate(db, per_page);
        let num_pages = paginator.num_pages().await?;
        paginator
            .fetch_page(page.saturating_sub(1))
            .await
            .map(|logs| (logs, num_pages))
    }

    /// Filtra por tipo de ação (create, update, delete, etc.)
    pub async fn find_by_action(
        db: &DbConn,
        action: &str,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<audit_logs::Model>, u64), DbErr> {
        let paginator = AuditLog::find()
            .filter(audit_logs::Column::Action.eq(action))
            .order_by_desc(audit_logs::Column::CreatedAt)
            .paginate(db, per_page);
        let num_pages = paginator.num_pages().await?;
        paginator
            .fetch_page(page.saturating_sub(1))
            .await
            .map(|logs| (logs, num_pages))
    }

    /// Filtra por tipo de entidade (user, post, etc.)
    pub async fn find_by_entity_type(
        db: &DbConn,
        entity_type: &str,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<audit_logs::Model>, u64), DbErr> {
        let paginator = AuditLog::find()
            .filter(audit_logs::Column::EntityType.eq(entity_type))
            .order_by_desc(audit_logs::Column::CreatedAt)
            .paginate(db, per_page);
        let num_pages = paginator.num_pages().await?;
        paginator
            .fetch_page(page.saturating_sub(1))
            .await
            .map(|logs| (logs, num_pages))
    }

    /// Filtra por range de data
    pub async fn find_by_date_range(
        db: &DbConn,
        start: chrono::DateTime<chrono::FixedOffset>,
        end: chrono::DateTime<chrono::FixedOffset>,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<audit_logs::Model>, u64), DbErr> {
        let paginator = AuditLog::find()
            .filter(audit_logs::Column::CreatedAt.gte(start))
            .filter(audit_logs::Column::CreatedAt.lte(end))
            .order_by_desc(audit_logs::Column::CreatedAt)
            .paginate(db, per_page);
        let num_pages = paginator.num_pages().await?;
        paginator
            .fetch_page(page.saturating_sub(1))
            .await
            .map(|logs| (logs, num_pages))
    }

    /// Busca logs de um usuário específico
    pub async fn find_by_user_id(
        db: &DbConn,
        user_id: uuid::Uuid,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<audit_logs::Model>, u64), DbErr> {
        let paginator = AuditLog::find()
            .filter(audit_logs::Column::UserId.eq(user_id))
            .order_by_desc(audit_logs::Column::CreatedAt)
            .paginate(db, per_page);
        let num_pages = paginator.num_pages().await?;
        paginator
            .fetch_page(page.saturating_sub(1))
            .await
            .map(|logs| (logs, num_pages))
    }

    /// Busca com filtros combinados (ação, entidade, usuário, data)
    /// Busca com filtros combinados (ação, entidade, usuário, data)
    pub async fn find_with_filters(
        db: &DbConn,
        filters: AuditLogFilters<'_>,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<audit_logs::Model>, u64), DbErr> {
        let mut query = AuditLog::find();

        if let Some(action) = filters.action {
            query = query.filter(audit_logs::Column::Action.eq(action));
        }
        if let Some(entity_type) = filters.entity_type {
            query = query.filter(audit_logs::Column::EntityType.eq(entity_type));
        }
        if let Some(user_id) = filters.user_id {
            query = query.filter(audit_logs::Column::UserId.eq(user_id));
        }
        if let Some(date_from) = filters.date_from {
            let start = date_from.and_hms_opt(0, 0, 0).unwrap();
            query = query.filter(audit_logs::Column::CreatedAt.gte(start));
        }
        if let Some(date_to) = filters.date_to {
            let end = date_to.and_hms_opt(23, 59, 59).unwrap();
            query = query.filter(audit_logs::Column::CreatedAt.lte(end));
        }

        let paginator = query
            .order_by_desc(audit_logs::Column::CreatedAt)
            .paginate(db, per_page);
        let num_pages = paginator.num_pages().await?;
        paginator
            .fetch_page(page.saturating_sub(1))
            .await
            .map(|logs| (logs, num_pages))
    }

    /// Busca TODOS os logs com filtros (para exportação CSV, sem paginação)
    /// Busca TODOS os logs com filtros (para exportação CSV, sem paginação)
    pub async fn find_all_with_filters(
        db: &DbConn,
        filters: AuditLogFilters<'_>,
    ) -> Result<Vec<audit_logs::Model>, DbErr> {
        let mut query = AuditLog::find();

        if let Some(action) = filters.action {
            query = query.filter(audit_logs::Column::Action.eq(action));
        }
        if let Some(entity_type) = filters.entity_type {
            query = query.filter(audit_logs::Column::EntityType.eq(entity_type));
        }
        if let Some(user_id) = filters.user_id {
            query = query.filter(audit_logs::Column::UserId.eq(user_id));
        }
        if let Some(date_from) = filters.date_from {
            let start = date_from.and_hms_opt(0, 0, 0).unwrap();
            query = query.filter(audit_logs::Column::CreatedAt.gte(start));
        }
        if let Some(date_to) = filters.date_to {
            let end = date_to.and_hms_opt(23, 59, 59).unwrap();
            query = query.filter(audit_logs::Column::CreatedAt.lte(end));
        }

        query
            .order_by_desc(audit_logs::Column::CreatedAt)
            .all(db)
            .await
    }

    /// Conta total de logs por ação
    pub async fn count_by_action(db: &DbConn, action: &str) -> Result<u64, DbErr> {
        AuditLog::find()
            .filter(audit_logs::Column::Action.eq(action))
            .count(db)
            .await
    }
}
