use std::path::PathBuf;

use axum::{Router, response::Redirect, routing::get};
use tower_http::services::{ServeDir, ServeFile};

use crate::state::AppState;

/// 构建公开静态资源路由
pub fn router(game_data_asset_dir: PathBuf) -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        // 显式提供站点图标，供浏览器和 RSS 阅读器读取
        .route_service("/favicon.ico", ServeFile::new("assets/favicon.ico"))
        .nest_service("/assets/game-data", ServeDir::new(game_data_asset_dir))
        .nest_service("/assets", ServeDir::new("assets"))
}

/// 在独立前端移除后将站点根路径引导至 API 文档
async fn root() -> Redirect {
    Redirect::permanent("/scalar")
}
