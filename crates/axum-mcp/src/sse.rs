use axum::{http::StatusCode, response::IntoResponse};

pub async fn handler() -> impl IntoResponse {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        "SSE is not supported on this server",
    )
}
