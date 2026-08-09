use axum::{Router, http::StatusCode, routing::get};

use crate::state::AppState;

/// 构建存活探针路由
pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/healthz", get(healthz))
}

/// 确认服务已可接收 HTTP 请求
async fn healthz() -> StatusCode {
    StatusCode::NO_CONTENT
}
