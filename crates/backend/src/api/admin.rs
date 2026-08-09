use axum::Router;

use crate::{
    features::{characters, news, workers},
    state::AppState,
};

/// 构建需要认证的管理路由
pub(crate) fn router() -> Router<AppState> {
    Router::new().nest(
        "/api/v1/admin",
        Router::new()
            .merge(workers::admin_router())
            .merge(news::admin_router())
            .merge(characters::admin_router()),
    )
}
