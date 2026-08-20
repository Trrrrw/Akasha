use axum::Router;

use crate::{
    features::{game_data, news, workers},
    state::AppState,
};

/// 构建需要认证的管理路由
pub(crate) fn router() -> Router<AppState> {
    Router::new().nest(
        "/api/v1/admin",
        Router::new()
            .merge(workers::admin_router())
            .merge(news::admin_router())
            .merge(game_data::admin_router()),
    )
}
