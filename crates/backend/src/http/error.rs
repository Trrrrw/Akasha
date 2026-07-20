use axum::{Json, http::StatusCode, response::IntoResponse};

use crate::http::response::ErrorResponse;

pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    NotFound(String),
    NotImplemented(String),
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::BadRequest(message) => {
                tracing::debug!(error.message = %message, "bad request");
                (StatusCode::BAD_REQUEST, message)
            }
            AppError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message),
            AppError::Forbidden(message) => (StatusCode::FORBIDDEN, message),
            AppError::Conflict(message) => (StatusCode::CONFLICT, message),
            AppError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            AppError::NotImplemented(message) => {
                tracing::warn!(error.message = %message, "not implemented endpoint called");
                (StatusCode::NOT_IMPLEMENTED, message)
            }
            AppError::Internal(err) => {
                tracing::error!(
                    error = ?err,
                    "internal server error"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        (status, Json(ErrorResponse::new(message))).into_response()
    }
}
