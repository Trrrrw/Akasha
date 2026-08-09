pub(crate) mod admin;
mod dto;
pub(crate) mod endpoints;
mod query;

use axum::{Router, routing::post};
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

/// 构建公开角色查询路由
pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    use utoipa_axum::routes;

    OpenApiRouter::new().routes(routes!(endpoints::list))
}

/// 构建受信任角色同步路由
pub(crate) fn admin_router() -> Router<AppState> {
    Router::new().route("/characters/sync", post(admin::sync))
}
