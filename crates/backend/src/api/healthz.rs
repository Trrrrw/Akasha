use axum::{Router, http::StatusCode, routing::get};

use crate::state::AppState;

/// 提供容器就绪检查
pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/healthz", get(healthz))
}

/// 确认服务已可接收请求
async fn healthz() -> StatusCode {
    StatusCode::NO_CONTENT
}
