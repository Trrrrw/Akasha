use anyhow::Result;
use axum::Router;

use crate::{
    Config,
    api::{self, healthz},
    http::middleware,
    site,
    state::AppState,
};

/// 构建配置完整的 Axum router 及共享应用状态
pub(crate) async fn build(config: Config) -> Result<Router> {
    let state = AppState::new(config).await?;
    let game_data_asset_dir = state.config().game_data_asset_dir.clone();

    let (v1_router, openapi) = api::v1::router();

    let router = Router::new()
        .merge(healthz::router())
        .merge(site::router(game_data_asset_dir))
        .merge(api::docs::router(openapi))
        .merge(api::admin::router())
        .merge(v1_router)
        .with_state(state);

    Ok(middleware::apply(router))
}
