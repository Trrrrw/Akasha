pub(crate) mod admin;
mod dto;
pub(crate) mod endpoints;
mod query;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, patch, put},
};
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

const GAME_DATA_SYNC_BODY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
const GAME_DATA_ASSET_BODY_LIMIT_BYTES: usize = 16 * 1024 * 1024;

/// 构建公开游戏数据路由
pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    use utoipa_axum::routes;

    OpenApiRouter::new()
        .routes(routes!(endpoints::collections))
        .routes(routes!(endpoints::list))
        .routes(routes!(endpoints::detail))
}

/// 构建游戏数据同步及资源上传路由
pub(crate) fn admin_router() -> Router<AppState> {
    Router::new()
        .merge(
            Router::new()
                .route(
                    "/games/{game_id}/data/{collection}/raw",
                    get(admin::list_raw),
                )
                .route(
                    "/games/{game_id}/data/{collection}",
                    patch(admin::update_collection).put(admin::sync_collection),
                )
                .layer(DefaultBodyLimit::max(GAME_DATA_SYNC_BODY_LIMIT_BYTES)),
        )
        .merge(
            Router::new()
                .route(
                    "/games/{game_id}/data/assets/{*path}",
                    put(admin::upload_asset),
                )
                .layer(DefaultBodyLimit::max(GAME_DATA_ASSET_BODY_LIMIT_BYTES)),
        )
}
