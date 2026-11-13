mod achievements;
pub(crate) mod admin;
pub(crate) mod endpoints;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, patch},
};
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

const GAME_DATA_SYNC_BODY_LIMIT_BYTES: usize = 64 * 1024 * 1024;

/// 构建公开游戏数据路由
pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    use utoipa_axum::routes;

    OpenApiRouter::new()
        .routes(routes!(achievements::list))
        .routes(routes!(achievements::groups))
        .routes(routes!(endpoints::ys_characters))
        .routes(routes!(endpoints::sr_characters))
        .routes(routes!(endpoints::zzz_characters))
}

/// 构建游戏数据同步及资源上传路由
pub(crate) fn admin_router() -> Router<AppState> {
    Router::new().merge(
        Router::new()
            .route(
                "/games/{game_id}/data/{collection}/sync-state",
                get(admin::list_sync_state),
            )
            .route(
                "/games/{game_id}/data/{collection}",
                patch(admin::update_collection).put(admin::sync_collection),
            )
            .layer(DefaultBodyLimit::max(GAME_DATA_SYNC_BODY_LIMIT_BYTES)),
    )
}
