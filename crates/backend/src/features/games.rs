mod dto;
pub(crate) mod endpoints;
mod version_admin;
mod versions;

use axum::{
    Router,
    routing::{get, put},
};
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

/// 构建公开游戏查询路由
pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    use utoipa_axum::routes;

    OpenApiRouter::new()
        .routes(routes!(endpoints::list))
        .routes(routes!(endpoints::detail))
        .routes(routes!(versions::list))
}

/// 构建游戏版本投影的受保护管理路由
pub(crate) fn admin_router() -> Router<AppState> {
    Router::new()
        .route(
            "/games/{game_id}/versions/raw",
            get(version_admin::list_raw),
        )
        .route("/games/{game_id}/versions", put(version_admin::sync))
}
