use crate::handlers::error_handler::{
    bad_request_response, forbidden_response, internal_error_response, unauthorized_response,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum AppError {
    InternalServerError(String),
    BadRequest(String),
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    ValidationErrors(validator::ValidationErrors),
    DbError(sea_orm::DbErr),
    Anyhow(anyhow::Error),
    Legacy(StatusCode, String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::InternalServerError(msg) => {
                tracing::error!("Internal Server Error: {}", msg);
                internal_error_response().into_response()
            }
            AppError::BadRequest(msg) => bad_request_response(msg).into_response(),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg).into_response(), // TODO: Use handler_404 if possible, but it requires Uri
            AppError::Unauthorized(msg) => {
                tracing::warn!("Unauthorized: {}", msg);
                unauthorized_response().into_response()
            }
            AppError::Forbidden(msg) => forbidden_response(msg).into_response(),
            AppError::ValidationErrors(errs) => {
                let msg = errs
                    .field_errors()
                    .iter()
                    .flat_map(|(_, errs)| {
                        errs.iter()
                            .filter_map(|e| e.message.as_ref().map(|m| m.to_string()))
                    })
                    .collect::<Vec<String>>()
                    .join("; ");
                bad_request_response(msg).into_response()
            }
            AppError::DbError(err) => {
                tracing::error!("Database Error: {}", err);
                internal_error_response().into_response()
            }
            AppError::Anyhow(err) => {
                tracing::error!("Unexpected Error: {}", err);
                internal_error_response().into_response()
            }
            AppError::Legacy(code, msg) => (code, msg).into_response(),
        }
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        AppError::DbError(err)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Anyhow(err)
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::ValidationErrors(err)
    }
}

impl From<(StatusCode, String)> for AppError {
    fn from(err: (StatusCode, String)) -> Self {
        AppError::Legacy(err.0, err.1)
    }
}
