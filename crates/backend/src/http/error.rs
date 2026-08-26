use akasha_application::ApplicationError;
use akasha_mys::MysError;
use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::IntoResponse,
};

use crate::http::{rate_limit::RateLimitExceeded, response::ErrorResponse};

/// HTTP 交付层可映射为统一错误响应的错误类型
#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Conflict(String),
    NotFound(String),
    TooManyRequests { retry_after_seconds: u64 },
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    /// 将应用错误编码为统一的 JSON HTTP 错误响应
    fn into_response(self) -> axum::response::Response {
        let (status, message, retry_after_seconds) = match self {
            AppError::BadRequest(message) => {
                tracing::debug!(error.message = %message, "bad request");
                (StatusCode::BAD_REQUEST, message, None)
            }
            AppError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message, None),
            AppError::Conflict(message) => (StatusCode::CONFLICT, message, None),
            AppError::NotFound(message) => (StatusCode::NOT_FOUND, message, None),
            AppError::TooManyRequests {
                retry_after_seconds,
            } => {
                tracing::debug!(retry_after_seconds, "public endpoint rate limit exceeded");
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    "rate limit exceeded".to_owned(),
                    Some(retry_after_seconds),
                )
            }
            AppError::Internal(err) => {
                tracing::error!(
                    error = ?err,
                    "internal server error"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                    None,
                )
            }
        };

        let mut response = (status, Json(ErrorResponse::new(message))).into_response();
        if let Some(retry_after_seconds) = retry_after_seconds {
            response.headers_mut().insert(
                header::RETRY_AFTER,
                HeaderValue::from_str(&retry_after_seconds.to_string())
                    .expect("重试秒数应为有效 HTTP 请求头"),
            );
        }
        response
    }
}

impl From<RateLimitExceeded> for AppError {
    /// 将客户端限流结果映射为 HTTP 429 响应
    fn from(error: RateLimitExceeded) -> Self {
        Self::TooManyRequests {
            retry_after_seconds: error.retry_after_seconds,
        }
    }
}

impl From<ApplicationError> for AppError {
    /// 将应用错误映射为本 API 暴露的 HTTP 状态语义
    fn from(error: ApplicationError) -> Self {
        match error {
            ApplicationError::InvalidInput(message) => Self::BadRequest(message),
            ApplicationError::Conflict(message) => Self::Conflict(message),
            ApplicationError::InvariantViolation(message) => {
                Self::Internal(anyhow::anyhow!(message))
            }
            ApplicationError::Repository(error) => Self::Internal(error.into()),
        }
    }
}

impl From<MysError> for AppError {
    /// 将米游社视频签名错误映射为统一的内部错误
    fn from(error: MysError) -> Self {
        Self::Internal(error.into())
    }
}

#[cfg(test)]
mod tests {
    use axum::{http::StatusCode, response::IntoResponse};

    use super::AppError;

    /// 限流错误返回 429 并携带标准重试秒数
    #[test]
    fn rate_limit_error_includes_retry_after_header() {
        let response = AppError::TooManyRequests {
            retry_after_seconds: 7,
        }
        .into_response();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()["retry-after"], "7");
    }
}
