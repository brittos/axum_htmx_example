use crate::handlers::auth_templates::{ForgotPasswordTemplate, ResetPasswordTemplate};
use crate::handlers::response::HtmlTemplate; // Use HtmlTemplate for standalone pages
use crate::middleware::csrf::get_csrf_token;
use crate::state::AppState;
use crate::utils::password::hash_password;
use axum::{
    extract::{Form, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Deserialize;
use tower_cookies::Cookies;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ForgotPasswordParams {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordPageParams {
    pub token: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordSubmitParams {
    pub token: String,
    pub password: String,
    pub confirm_password: String,
}

/// GET /auth/forgot-password
pub async fn forgot_password_page(cookies: Cookies) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&cookies);
    HtmlTemplate(ForgotPasswordTemplate {
        error_message: None,
        success_message: None,
        csrf_token,
    })
}

/// POST /auth/forgot-password
pub async fn forgot_password_submit(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(params): Form<ForgotPasswordParams>,
) -> impl IntoResponse {
    use entity::{password_resets, users};

    let csrf_token = get_csrf_token(&cookies);

    // 1. Check if user exists
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&params.email))
        .one(&state.conn)
        .await;

    if let Ok(Some(user)) = user {
        // 2. Generate Token
        let token = Uuid::now_v7().to_string();
        let expires_at = Utc::now() + Duration::minutes(30); // 30 min expiry

        // 3. Store in DB (Tx to clean old tokens)
        let txn = state.conn.begin().await.unwrap();

        // Delete old tokens for this user
        let _ = password_resets::Entity::delete_many()
            .filter(password_resets::Column::UserId.eq(user.id))
            .exec(&txn)
            .await;

        // Create new token
        let reset_model = password_resets::ActiveModel {
            id: Set(Uuid::now_v7()),
            user_id: Set(user.id),
            token: Set(token.clone()),
            expires_at: Set(expires_at.into()),
            created_at: Set(Utc::now().into()),
        };

        let _ = reset_model.insert(&txn).await;

        txn.commit().await.unwrap();

        // 4. Mock Send Email
        tracing::info!(
            "PASSWORD RECOVERY: Send email to {} with link: http://localhost:8080/auth/reset-password?token={}",
            params.email,
            token
        );
    } else {
        // Prevent enumeration: pretend we sent it
        tracing::info!(
            "PASSWORD RECOVERY: Email {} not found, but simulating success.",
            params.email
        );
    }

    HtmlTemplate(ForgotPasswordTemplate {
        error_message: None,
        success_message: Some(
            "If an account exists with this email, you will receive a password reset link shortly."
                .to_string(),
        ),
        csrf_token,
    })
}

/// GET /auth/reset-password
pub async fn reset_password_page(
    State(state): State<AppState>,
    cookies: Cookies,
    Query(params): Query<ResetPasswordPageParams>,
) -> impl IntoResponse {
    use entity::password_resets;

    let csrf_token = get_csrf_token(&cookies);

    // Verify token
    let reset = password_resets::Entity::find()
        .filter(password_resets::Column::Token.eq(&params.token))
        .one(&state.conn)
        .await;

    match reset {
        Ok(Some(r)) => {
            if r.expires_at < Utc::now() {
                return HtmlTemplate(ForgotPasswordTemplate {
                    error_message: Some(
                        "This link has expired. Please request a new one.".to_string(),
                    ),
                    success_message: None,
                    csrf_token,
                })
                .into_response();
            }

            HtmlTemplate(ResetPasswordTemplate {
                token: params.token,
                error_message: None,
                csrf_token,
            })
            .into_response()
        }
        _ => HtmlTemplate(ForgotPasswordTemplate {
            error_message: Some("Invalid password reset link.".to_string()),
            success_message: None,
            csrf_token,
        })
        .into_response(),
    }
}

/// POST /auth/reset-password
pub async fn reset_password_submit(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(params): Form<ResetPasswordSubmitParams>,
) -> impl IntoResponse {
    use entity::{password_resets, users};

    let csrf_token = get_csrf_token(&cookies);

    // 1. Validate Passwords
    if params.password != params.confirm_password {
        return HtmlTemplate(ResetPasswordTemplate {
            token: params.token,
            error_message: Some("Passwords do not match.".to_string()),
            csrf_token,
        })
        .into_response();
    }

    if params.password.len() < 8 {
        return HtmlTemplate(ResetPasswordTemplate {
            token: params.token,
            error_message: Some("Password must be at least 8 characters.".to_string()),
            csrf_token,
        })
        .into_response();
    }

    // 2. Verify and Consume Token
    let txn = match state.conn.begin().await {
        Ok(t) => t,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let reset = password_resets::Entity::find()
        .filter(password_resets::Column::Token.eq(&params.token))
        .one(&txn)
        .await;

    if let Ok(Some(r)) = reset {
        if r.expires_at < Utc::now() {
            return HtmlTemplate(ForgotPasswordTemplate {
                error_message: Some("Link expired.".to_string()),
                success_message: None,
                csrf_token,
            })
            .into_response();
        }

        // 3. Update User Password
        let new_hash = match hash_password(&params.password) {
            Ok(h) => h,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        // We need to fetch the user to convert to ActiveModel or update by ID
        // sea_orm allows update specific columns by ID if we construct ActiveModel correctly?
        // Easiest is find user.
        let user = users::Entity::find_by_id(r.user_id)
            .one(&txn)
            .await
            .unwrap(); // Should exist due to FK

        if let Some(user) = user {
            let mut active_user = user.into_active_model();
            active_user.password = Set(new_hash);
            let _ = active_user.update(&txn).await;

            // 4. Delete Token
            let _ = r.delete(&txn).await;

            txn.commit().await.unwrap();

            return Redirect::to("/admin/login?reset_success=true").into_response();
        }
    }

    HtmlTemplate(ForgotPasswordTemplate {
        error_message: Some("Invalid or expired token.".to_string()),
        success_message: None,
        csrf_token,
    })
    .into_response()
}
