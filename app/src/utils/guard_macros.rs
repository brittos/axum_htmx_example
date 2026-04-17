//! Macros de segurança (Guards) para o projeto.

/// Macro para verificar permissão RBAC em handlers.
///
/// Verifica autenticação e autorização, retornando erro apropriado se falhar.
/// Retorna o `user_id` do usuário autenticado se a verificação passar.
///
/// # Uso
/// ```rust,no_run
/// use axum::{extract::State, http::StatusCode, response::IntoResponse};
/// use tower_cookies::Cookies;
/// use axum_example_app::require_permission;
/// use axum_example_app::state::AppState;
///
/// pub async fn admin_users(
///     State(state): State<AppState>,
///     cookies: Cookies,
/// ) -> Result<impl IntoResponse, (StatusCode, String)> {
///     let user_id = require_permission!(state, cookies, "User Management", "read");
///     Ok("Success")
/// }
/// ```
#[macro_export]
macro_rules! require_permission {
    ($state:expr, $cookies:expr, $resource:expr, $action:expr) => {{
        use axum::http::StatusCode;
        use $crate::middleware::auth::get_current_user_id;
        use $crate::service::rbac_service::check_permission;

        // 1. Verificar autenticação
        let user_id = match get_current_user_id(&$cookies, &$state).await {
            Some(id) => id,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Sessão expirada. Por favor, faça login novamente.".to_string(),
                )
                    .into());
            }
        };

        // 2. Verificar autorização
        let mut redis_conn = $state.redis.clone();
        if !check_permission(&$state.conn, &mut redis_conn, user_id, $resource, $action).await {
            tracing::warn!(
                "Access denied: user {} tried to {} on {}",
                user_id,
                $action,
                $resource
            );

            // Enviar toast de warning para o usuário
            $crate::handlers::admin::toasts::send_toast(
                &$state,
                user_id,
                $crate::state::ToastLevel::Warning,
                format!(
                    "Acesso negado: você não tem permissão para {} em {}",
                    $action, $resource
                ),
            );

            return Err((
                StatusCode::FORBIDDEN,
                format!("Você não tem permissão para {} em {}.", $action, $resource),
            )
                .into());
        }

        user_id
    }};
}

/// Macro simplificada que apenas verifica permissão sem retornar user_id.
/// Útil quando o user_id não é necessário no handler.
#[macro_export]
macro_rules! check_permission_or_403 {
    ($state:expr, $cookies:expr, $resource:expr, $action:expr) => {{
        let _ = $crate::require_permission!($state, $cookies, $resource, $action);
    }};
}
