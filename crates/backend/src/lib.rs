mod api;
mod app;
mod config;
mod features;
mod http;
mod site;
mod state;

pub use config::Config;

use anyhow::Result;
use axum::Router;

/// 构建完整 HTTP 应用
pub async fn build_app(config: Config) -> Result<Router> {
    app::build(config).await
}
