//! Handlers de Audit Logs.

use crate::dto::AuditLogsQuery;
use crate::handlers::admin_templates::{AuditLogsFullTemplate, AuditLogsPartialTemplate};
use crate::handlers::response::{HtmlTemplate, render_page};
use crate::middleware::get_sidebar_user;
use crate::models::view::AuditLogViewModel;
use crate::require_permission;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use chrono::NaiveDate;
use tower_cookies::Cookies;

/// Helper para converter Option<String> vazia em None
fn non_empty(opt: Option<String>) -> Option<String> {
    opt.filter(|s| !s.is_empty())
}

/// Parse date string (YYYY-MM-DD) para NaiveDate
fn parse_date(s: Option<String>) -> Option<NaiveDate> {
    s.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
}

/// Struct para representar usuário no dropdown
#[derive(Clone)]
pub struct UserOption {
    pub id: String,
    pub name: String,
}

/// Busca user_ids que correspondem ao username (busca parcial)
async fn find_user_ids_by_username(
    conn: &sea_orm::DbConn,
    username_search: &str,
) -> Vec<uuid::Uuid> {
    use entity::users;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let pattern = format!("%{}%", username_search.to_lowercase());
    users::Entity::find()
        .filter(users::Column::Username.contains(&pattern))
        .all(conn)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|u| {
            u.username
                .to_lowercase()
                .contains(&username_search.to_lowercase())
        })
        .map(|u| u.id)
        .collect()
}

/// Handler para página de audit logs
pub async fn audit_logs_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    cookies: Cookies,
    Query(params): Query<AuditLogsQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::repository::AuditQuery;
    use entity::users;
    use sea_orm::EntityTrait;
    use std::collections::HashMap;

    // Verificar permissão: Audit Logs - read
    let _user_id = require_permission!(state, cookies, "Audit Logs", "read");

    let page = params.page.unwrap_or(1).max(1);
    let per_page = 20u64;

    let action = non_empty(params.action);
    let entity_type = non_empty(params.entity_type);
    let username_filter = non_empty(params.username);
    let date_from = parse_date(non_empty(params.date_from));
    let date_to = parse_date(non_empty(params.date_to));

    // Se tiver busca por username, encontrar os user_ids correspondentes
    let user_id_filter: Option<uuid::Uuid> = if let Some(ref username) = username_filter {
        let matching_ids = find_user_ids_by_username(&state.conn, username).await;
        // Se encontrou apenas um, usar; senão None (busca muito ampla ou não encontrada)
        if matching_ids.len() == 1 {
            Some(matching_ids[0])
        } else {
            None // Retornar todos se busca não der match único
        }
    } else {
        None
    };

    let (logs_db, total_pages) = AuditQuery::find_with_filters(
        &state.conn,
        crate::repository::AuditLogFilters {
            action: action.as_deref(),
            entity_type: entity_type.as_deref(),
            user_id: user_id_filter,
            date_from,
            date_to,
        },
        page,
        per_page,
    )
    .await
    .unwrap_or((vec![], 1));

    // Buscar todos os usuários (não mais necessário para dropdown, mas mantemos por compatibilidade)
    let all_users: Vec<UserOption> = users::Entity::find()
        .all(&state.conn)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|u| UserOption {
            id: u.id.to_string(),
            name: u.username,
        })
        .collect();

    // Coletar user_ids únicos para buscar usernames em batch
    let user_ids: Vec<uuid::Uuid> = logs_db
        .iter()
        .filter_map(|log| log.user_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Buscar usernames
    let mut usernames: HashMap<uuid::Uuid, String> = HashMap::new();
    if !user_ids.is_empty() {
        let users_list = users::Entity::find()
            .all(&state.conn)
            .await
            .unwrap_or_default();
        for user in users_list {
            if user_ids.contains(&user.id) {
                usernames.insert(user.id, user.username);
            }
        }
    }

    // Converter logs e preencher usernames
    let logs: Vec<AuditLogViewModel> = logs_db
        .into_iter()
        .map(|m| {
            let mut vm: AuditLogViewModel = m.clone().into();
            if let Some(uid) = m.user_id {
                vm.username = usernames.get(&uid).cloned();
            }
            vm
        })
        .collect();

    let sidebar_user = get_sidebar_user(&cookies, &state).await;

    Ok(render_page(
        &headers,
        AuditLogsFullTemplate {
            title: "Audit Logs".into(),
            page_title: "Audit Logs".into(),
            active_tab: "audit".into(),
            logs: logs.clone(),
            current_page: page,
            total_pages: total_pages.max(1),
            filter_action: action.clone(),
            filter_entity_type: entity_type.clone(),
            filter_username: username_filter.clone(),
            filter_date_from: date_from.map(|d| d.to_string()),
            filter_date_to: date_to.map(|d| d.to_string()),
            users: all_users.clone(),
            sidebar_user,
        },
        AuditLogsPartialTemplate {
            logs,
            current_page: page,
            total_pages: total_pages.max(1),
            filter_action: action,
            filter_entity_type: entity_type,
            filter_username: username_filter,
            filter_date_from: date_from.map(|d| d.to_string()),
            filter_date_to: date_to.map(|d| d.to_string()),
            users: all_users,
        },
        "Bero Admin | Audit Logs",
    ))
}

/// Handler para partial de audit logs (HTMX)
pub async fn audit_logs_partial_handler(
    State(state): State<AppState>,
    Query(params): Query<AuditLogsQuery>,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::repository::AuditQuery;
    use entity::users;
    use sea_orm::EntityTrait;
    use std::collections::HashMap;

    // Verificar permissão
    let _user_id = require_permission!(state, cookies, "Audit Logs", "read");

    let page = params.page.unwrap_or(1).max(1);
    let per_page = 20u64;

    let action = non_empty(params.action);
    let entity_type = non_empty(params.entity_type);
    let username_filter = non_empty(params.username);
    let date_from = parse_date(non_empty(params.date_from));
    let date_to = parse_date(non_empty(params.date_to));

    // Se tiver busca por username, encontrar os user_ids correspondentes
    let user_id_filter: Option<uuid::Uuid> = if let Some(ref username) = username_filter {
        let matching_ids = find_user_ids_by_username(&state.conn, username).await;
        if matching_ids.len() == 1 {
            Some(matching_ids[0])
        } else {
            None
        }
    } else {
        None
    };

    let (logs_db, total_pages) = AuditQuery::find_with_filters(
        &state.conn,
        crate::repository::AuditLogFilters {
            action: action.as_deref(),
            entity_type: entity_type.as_deref(),
            user_id: user_id_filter,
            date_from,
            date_to,
        },
        page,
        per_page,
    )
    .await
    .unwrap_or((vec![], 1));

    // Buscar usuarios
    let all_users: Vec<UserOption> = users::Entity::find()
        .all(&state.conn)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|u| UserOption {
            id: u.id.to_string(),
            name: u.username,
        })
        .collect();

    // Coletar user_ids únicos
    let user_ids: Vec<uuid::Uuid> = logs_db
        .iter()
        .filter_map(|log| log.user_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Buscar usernames
    let mut usernames: HashMap<uuid::Uuid, String> = HashMap::new();
    if !user_ids.is_empty() {
        let users_list = users::Entity::find()
            .all(&state.conn)
            .await
            .unwrap_or_default();
        for user in users_list {
            if user_ids.contains(&user.id) {
                usernames.insert(user.id, user.username);
            }
        }
    }

    // Converter e preencher usernames
    let logs: Vec<AuditLogViewModel> = logs_db
        .into_iter()
        .map(|m| {
            let mut vm: AuditLogViewModel = m.clone().into();
            if let Some(uid) = m.user_id {
                vm.username = usernames.get(&uid).cloned();
            }
            vm
        })
        .collect();

    let mut res = HtmlTemplate(AuditLogsPartialTemplate {
        logs,
        current_page: page,
        total_pages: total_pages.max(1),
        filter_action: action,
        filter_entity_type: entity_type,
        filter_username: username_filter,
        filter_date_from: date_from.map(|d| d.to_string()),
        filter_date_to: date_to.map(|d| d.to_string()),
        users: all_users,
    })
    .into_response();

    res.headers_mut()
        .insert("HX-Title", "Bero Admin | Audit Logs".parse().unwrap());
    Ok(res)
}

/// Handler para exportar audit logs em CSV
pub async fn audit_logs_export_csv_handler(
    State(state): State<AppState>,
    Query(params): Query<AuditLogsQuery>,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use crate::repository::AuditQuery;
    use entity::users;
    use sea_orm::EntityTrait;
    use std::collections::HashMap;

    // Verificar permissão
    let _user_id = require_permission!(state, cookies, "Audit Logs", "read");

    let action = non_empty(params.action);
    let entity_type = non_empty(params.entity_type);
    let username_filter = non_empty(params.username);
    let date_from = parse_date(non_empty(params.date_from));
    let date_to = parse_date(non_empty(params.date_to));

    // Se tiver busca por username, encontrar os user_ids correspondentes
    let user_id_filter: Option<uuid::Uuid> = if let Some(ref username) = username_filter {
        let matching_ids = find_user_ids_by_username(&state.conn, username).await;
        if matching_ids.len() == 1 {
            Some(matching_ids[0])
        } else {
            None
        }
    } else {
        None
    };

    // Buscar TODOS os logs (sem paginação)
    let logs_db = AuditQuery::find_all_with_filters(
        &state.conn,
        crate::repository::AuditLogFilters {
            action: action.as_deref(),
            entity_type: entity_type.as_deref(),
            user_id: user_id_filter,
            date_from,
            date_to,
        },
    )
    .await
    .unwrap_or_default();

    // Buscar usernames
    let users_list = users::Entity::find()
        .all(&state.conn)
        .await
        .unwrap_or_default();
    let usernames: HashMap<uuid::Uuid, String> =
        users_list.into_iter().map(|u| (u.id, u.username)).collect();

    // Gerar CSV
    let mut csv = String::from("Data/Hora,Ação,Entidade,ID Entidade,Usuário,IP,Detalhes\n");
    for log in logs_db {
        let username = log
            .user_id
            .and_then(|uid| usernames.get(&uid).cloned())
            .unwrap_or_else(|| "-".to_string());
        let entity_id = log
            .entity_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "-".to_string());
        let ip = log.ip_address.clone().unwrap_or_else(|| "-".to_string());
        let details = log
            .details
            .clone()
            .unwrap_or_default()
            .replace("\"", "\"\""); // Escape quotes for CSV
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            log.created_at.format("%Y-%m-%d %H:%M:%S"),
            log.action,
            log.entity_type,
            entity_id,
            username,
            ip,
            details
        ));
    }

    let filename = format!(
        "audit_logs_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    let content_disposition = format!("attachment; filename=\"{}\"", filename);

    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        csv,
    ))
}
