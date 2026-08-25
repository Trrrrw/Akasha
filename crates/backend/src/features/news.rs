pub(crate) mod admin;
pub(crate) mod dto;
pub(crate) mod endpoints;
mod nfo;
mod query;
mod rss;

use axum::{
    Router,
    routing::{get, post},
};
use chrono::FixedOffset;
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

/// 构建公开新闻查询和 RSS 路由
pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    use utoipa_axum::routes;

    OpenApiRouter::new()
        .routes(routes!(endpoints::list_sources))
        .routes(routes!(endpoints::list_tags))
        .routes(routes!(endpoints::list))
        .routes(routes!(endpoints::detail))
        .routes(routes!(endpoints::download_movie_nfo))
        .routes(routes!(endpoints::download_series_nfo))
        .routes(routes!(endpoints::download_episode_nfo))
        .routes(routes!(endpoints::video))
        .routes(routes!(endpoints::rss))
}

/// 构建受信任新闻同步路由
pub(crate) fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/news/update", post(admin::update_news))
        .route("/news/raw", get(admin::list_raw))
        .route("/news/tags/sync", post(admin::sync_tags))
        .route("/news/tags/update", post(admin::update_tags))
        .route("/news/characters/update", post(admin::update_characters))
}

/// 返回 NFO 本地日期计算使用的中国标准时区
pub(super) fn china_timezone() -> FixedOffset {
    FixedOffset::east_opt(8 * 60 * 60).expect("UTC+8 应为有效时区偏移")
}

/// 将数据库中的毫秒时长转换为最接近的整数秒
pub(super) fn video_duration_seconds(duration_ms: i64) -> Option<u64> {
    let duration_ms = u64::try_from(duration_ms).ok()?;
    Some((duration_ms + 500) / 1_000)
}
