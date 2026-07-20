use anyhow::Result;
use axum::Router;

use crate::{
    Config,
    api::{self, healthz},
    http::middleware,
    site,
    state::AppState,
};

pub(crate) async fn build(config: Config) -> Result<Router> {
    let state = AppState::new(config).await?;

    let (v1_router, openapi) = api::v1::router();

    let router = Router::new()
        .merge(healthz::router())
        .merge(site::router())
        .merge(api::docs::router(openapi))
        .merge(api::auth::router())
        .merge(api::admin::router())
        .merge(v1_router)
        .with_state(state);

    Ok(middleware::apply(router))
}
