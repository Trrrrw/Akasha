use akasha_application::news::{NewsItemResult, VideoPlayback};
use axum::{
    Json,
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;

use crate::{http::error::AppError, state::AppState};

use super::{
    dto::NewsVideoResponse,
    nfo,
    query::{
        NewsDetailPath, NewsEpisodeNfoQuery, NewsSeriesEpisodePath, NewsSeriesPath, NewsSourceQuery,
    },
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
    Path(NewsDetailPath { game_id, news_id }): Path<NewsDetailPath>,
    Query(query): Query<NewsSourceQuery>,
) -> Result<Response, AppError> {
    let source = query.into_source()?;
    let result = find_video_news(&state, &game_id, &source, &news_id).await?;

    // NFO 只描述媒体元数据，不解析会过期的米游社播放签名
    let document = nfo::build_movie(
        &game_id,
        &source,
        result.item,
        result.game_cover,
        &state.config().asset_base_url,
    )
    .map_err(AppError::Internal)?;

    nfo_file_response(document)
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
    Path(NewsSeriesPath { game_id, tag_name }): Path<NewsSeriesPath>,
    Query(query): Query<NewsSourceQuery>,
) -> Result<Response, AppError> {
    let source = query.into_source()?;
    let series = state
        .application()
        .find_news_series(&game_id, &source, &tag_name)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "video series {tag_name} not found in {source} {game_id}"
            ))
        })?;
    let document = nfo::build_series(&game_id, &source, series, &state.config().asset_base_url)
        .map_err(AppError::Internal)?;

    nfo_file_response(document)
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
    Path(NewsSeriesEpisodePath {
        game_id,
        tag_name,
        news_id,
    }): Path<NewsSeriesEpisodePath>,
    Query(query): Query<NewsEpisodeNfoQuery>,
) -> Result<Response, AppError> {
    let NewsEpisodeNfoQuery {
        source,
        season,
        episode,
    } = query.validate()?;
    let result = find_video_news(&state, &game_id, &source, &news_id).await?;
    if !result.item.tags.iter().any(|tag| tag == &tag_name) {
        return Err(AppError::NotFound(format!(
            "video news {news_id} does not belong to tag {tag_name}"
        )));
    }
    let series = state
        .application()
        .find_news_series(&game_id, &source, &tag_name)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "video series {tag_name} not found in {source} {game_id}"
            ))
        })?;
    let context = nfo::EpisodeNfoContext::new(&game_id, &source, series, season, episode);
    let document = nfo::build_episode(context, result.item, &state.config().asset_base_url)
        .map_err(AppError::Internal)?;

    nfo_file_response(document)
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
    Path(NewsDetailPath { game_id, news_id }): Path<NewsDetailPath>,
    Query(query): Query<NewsSourceQuery>,
) -> Result<Json<NewsVideoResponse>, AppError> {
    let source = query.into_source()?;
    // 在读取数据库或访问米游社前按安全解析的客户端 IP 消耗视频接口令牌
    let client_ip = state
        .public_rate_limiters()
        .client_ip(&headers, client_address);
    state.public_rate_limiters().check_video(client_ip)?;

    let result = find_video_news(&state, &game_id, &source, &news_id).await?;
    let item = result.item;
    let video_playback = item.video_playback.unwrap_or(VideoPlayback::Direct);

    // 米游社视频必须按文章 ID 请求最新签名，其他来源沿用数据库中的地址
    let video_url = if source == "mys" {
        state
            .mys_video_service()
            .resolve_video_url(&game_id, &news_id)
            .await?
    } else {
        item.video_url
    };
    let video_url = video_url
        .ok_or_else(|| AppError::NotFound(format!("video for news {news_id} is not available")))?;

    Ok(Json(NewsVideoResponse::new(video_url, video_playback)))
}

/// 查找一条视频新闻并统一映射不存在或类型不符的情况
async fn find_video_news(
    state: &AppState,
    game_id: &str,
    source_id: &str,
    news_id: &str,
) -> Result<NewsItemResult, AppError> {
    let result = state
        .application()
        .find_news(game_id, source_id, news_id)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("news {news_id} not found in {source_id} {game_id}"))
        })?;

    if result.item.news_type != "video" {
        return Err(AppError::NotFound(format!("news {news_id} is not a video")));
    }

    Ok(result)
}

/// 将 NFO 文档转换为带安全下载文件名的 XML 响应
fn nfo_file_response(document: nfo::NfoDocument) -> Result<Response, AppError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", document.filename))
            .map_err(|error| AppError::Internal(error.into()))?,
    );

    Ok((headers, document.xml).into_response())
}
