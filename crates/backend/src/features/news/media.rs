use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
};
use std::net::SocketAddr;

use crate::{http::error::AppError, state::AppState};

use super::{
    endpoints,
    query::{NewsDetailPath, NewsEpisodeNfoQuery, NewsSeriesEpisodePath, NewsSeriesPath, NewsSourceQuery},
};

#[utoipa::path(
    get,
    path = "/games/{game_id}/news/{news_id}/media/nfo",
    tag = "News",
    summary = "下载独立视频 NFO",
    description = "将一条视频新闻作为独立电影导出为 Kodi 和 Jellyfin Movie NFO",
    params(NewsDetailPath, NewsSourceQuery),
    responses(
        (status = 200, description = "视频新闻 NFO XML", content_type = "application/xml"),
        (status = 404, body = crate::http::response::ErrorResponse),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
pub(super) async fn movie_nfo(
    State(state): State<AppState>,
    Path(path): Path<NewsDetailPath>,
    Query(query): Query<NewsSourceQuery>,
) -> Result<impl IntoResponse, AppError> {
    endpoints::download_movie_nfo(State(state), Path(path), Query(query)).await
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/news/series/{tag_name}/media/nfo",
    tag = "News",
    summary = "下载标签剧集 NFO",
    description = "将至少包含一条视频的新闻标签导出为 tvshow.nfo，供前端交给媒体下载流程",
    params(NewsSeriesPath, NewsSourceQuery),
    responses(
        (status = 200, description = "标签剧集 TV Show NFO", content_type = "application/xml"),
        (status = 404, body = crate::http::response::ErrorResponse),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
pub(super) async fn series_nfo(
    State(state): State<AppState>,
    Path(path): Path<NewsSeriesPath>,
    Query(query): Query<NewsSourceQuery>,
) -> Result<impl IntoResponse, AppError> {
    endpoints::download_series_nfo(State(state), Path(path), Query(query)).await
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/news/series/{tag_name}/episodes/{news_id}/media/nfo",
    tag = "News",
    summary = "下载视频单集 NFO",
    description = "将标签内的视频导出为 episodedetails NFO，季集编号应与媒体文件名一致",
    params(NewsSeriesEpisodePath, NewsEpisodeNfoQuery),
    responses(
        (status = 200, description = "视频新闻 Episode NFO", content_type = "application/xml"),
        (status = 400, body = crate::http::response::ErrorResponse),
        (status = 404, body = crate::http::response::ErrorResponse),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
pub(super) async fn episode_nfo(
    State(state): State<AppState>,
    Path(path): Path<NewsSeriesEpisodePath>,
    Query(query): Query<NewsEpisodeNfoQuery>,
) -> Result<impl IntoResponse, AppError> {
    endpoints::download_episode_nfo(State(state), Path(path), Query(query)).await
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/news/{news_id}/media/video",
    tag = "News",
    summary = "获取新闻视频播放地址",
    description = "返回当前有效的视频播放地址，米游社地址会按新闻单独刷新签名；请求按客户端 IP 限流",
    params(NewsDetailPath, NewsSourceQuery),
    responses(
        (status = 200, body = super::dto::NewsVideoResponse),
        (
            status = 429,
            body = crate::http::response::ErrorResponse,
            headers(("Retry-After" = u64, description = "建议等待秒数"))
        ),
        (status = 404, body = crate::http::response::ErrorResponse),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
pub(super) async fn video(
    State(state): State<AppState>,
    ConnectInfo(client_address): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(path): Path<NewsDetailPath>,
    Query(query): Query<NewsSourceQuery>,
) -> Result<impl IntoResponse, AppError> {
    endpoints::video(
        State(state),
        ConnectInfo(client_address),
        headers,
        Path(path),
        Query(query),
    )
    .await
}
