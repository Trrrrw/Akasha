use akasha_application::news::NewsItemResult;
use axum::{
    Json,
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Query as MultiQuery;
use std::net::SocketAddr;

use crate::{
    features::news::{
        dto::{
            NewsDetailResponse, NewsItemResponse, NewsListMeta, NewsSourceResponse,
            NewsTagsResponse, NewsVideoResponse,
        },
        nfo,
        query::{
            NewsDetailPath, NewsEpisodeNfoQuery, NewsFilterQuery, NewsPageQuery, NewsRssQuery,
            NewsSeriesEpisodePath, NewsSeriesPath, NewsSourceQuery,
        },
        rss,
    },
    http::{
        error::AppError,
        path::{GamePath, require_game, require_news_source},
        response::{ListResponse, PageResponse},
    },
    mys::MysVideoUrlResolution,
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/games/{game_id}/news/sources",
    tag = "News",
    summary = "获取新闻来源列表",
    description = "返回指定游戏已经配置的新闻来源，来源 ID 可用于其他新闻接口",
    params(GamePath),
    responses(
        (status = 200, body = ListResponse<NewsSourceResponse>),
        (status = 404, body = crate::http::response::ErrorResponse),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
/// 列出一个游戏已配置的新闻来源
pub(super) async fn list_sources(
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
) -> Result<Json<ListResponse<NewsSourceResponse>>, AppError> {
    require_game(&state, &game_id).await?;
    let rows = state.application().list_news_sources(&game_id).await?;

    let items = rows
        .into_iter()
        .map(NewsSourceResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(ListResponse {
        total: items.len() as u64,
        items,
    }))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/news/tags",
    tag = "News",
    summary = "获取新闻标签列表",
    description = "返回指定来源的标签分组、新闻数量和最近新闻预览",
    params(GamePath, NewsSourceQuery),
    responses(
        (status = 200, body = NewsTagsResponse),
        (status = 404, body = crate::http::response::ErrorResponse),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
/// 列出一个来源的新闻标签及其最近新闻预览
pub(super) async fn list_tags(
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
    Query(query): Query<NewsSourceQuery>,
) -> Result<Json<NewsTagsResponse>, AppError> {
    let source = query.into_source()?;
    require_news_source(&state, &game_id, &source).await?;
    let result = state
        .application()
        .list_news_tags(&game_id, &source)
        .await?;

    Ok(Json(NewsTagsResponse::from_rows(
        game_id,
        source,
        result.tags,
        result.game_cover.as_deref(),
        &state.config().asset_base_url,
    )))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/news",
    tag = "News",
    summary = "获取新闻列表",
    description = "按来源、标题语法、标签、角色、类型和发布日期筛选新闻，并返回稳定分页结果",
    params(GamePath, NewsFilterQuery, NewsPageQuery),
    responses(
        (status = 200, body = PageResponse<NewsItemResponse, NewsListMeta>),
        (status = 400, body = crate::http::response::ErrorResponse),
        (status = 404, body = crate::http::response::ErrorResponse),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
/// 列出一个游戏来源经筛选的一页新闻
pub(super) async fn list(
    State(state): State<AppState>,
    Path(path): Path<GamePath>,
    MultiQuery(filter_query): MultiQuery<NewsFilterQuery>,
    Query(page_query): Query<NewsPageQuery>,
) -> Result<Json<PageResponse<NewsItemResponse, NewsListMeta>>, AppError> {
    let filter = page_query.apply(filter_query.into_filter(path)?)?;
    let source_id = filter.filter.source_id.clone();
    let game_id = filter.filter.game_id.clone();
    require_news_source(&state, &game_id, &source_id).await?;
    let limit = filter.limit;
    let offset = filter.offset;

    tracing::debug!(
        game_id = %game_id,
        source_id = %source_id,
        limit,
        offset,
        "listing news"
    );

    let result = state.application().list_news(filter).await?;
    let items = result
        .items
        .into_iter()
        .map(|news| {
            NewsItemResponse::from_summary(
                news,
                result.game_cover.as_deref(),
                &state.config().asset_base_url,
            )
        })
        .collect();

    Ok(Json(PageResponse {
        total: result.total,
        limit,
        offset,
        items,
        meta: NewsListMeta::new(source_id, game_id),
    }))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/news/{news_id}",
    tag = "News",
    summary = "获取新闻详情",
    description = "按游戏、来源和新闻 ID 返回完整公开信息，视频新闻同时包含最多 8 条相关推荐",
    params(NewsDetailPath, NewsSourceQuery),
    responses(
        (status = 200, body = NewsDetailResponse),
        (status = 404, body = crate::http::response::ErrorResponse),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
/// 返回请求游戏来源的一条新闻
pub(super) async fn detail(
    State(state): State<AppState>,
    Path(NewsDetailPath { game_id, news_id }): Path<NewsDetailPath>,
    Query(query): Query<NewsSourceQuery>,
) -> Result<Json<NewsDetailResponse>, AppError> {
    let source = query.into_source()?;
    let result = state
        .application()
        .find_news_detail(&game_id, &source, &news_id)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("news {news_id} not found in {source} {game_id}"))
        })?;

    Ok(Json(NewsDetailResponse::from_result(
        result,
        &state.config().asset_base_url,
    )))
}

/// 生成并下载一条独立视频新闻的 Movie NFO 文件
pub(super) async fn download_movie_nfo(
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

/// 生成并下载一个新闻标签的 TV Show NFO 文件
pub(super) async fn download_series_nfo(
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

/// 生成并下载标签内一条视频新闻的 Episode NFO 文件
pub(super) async fn download_episode_nfo(
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

/// 按新闻 ID 获取当前有效的视频播放地址
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

    Ok(Json(NewsVideoResponse::new(video_url)))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/news/rss",
    tag = "News",
    summary = "获取新闻 RSS",
    description = "按新闻筛选条件生成固定为发布时间倒序的 RSS 2.0 订阅源；请求按客户端 IP 限流，冷缓存签名刷新数量受请求预算限制",
    params(GamePath, NewsFilterQuery, NewsRssQuery),
    responses(
        (status = 200, description = "RSS 2.0 XML", content_type = "application/rss+xml"),
        (status = 400, body = crate::http::response::ErrorResponse),
        (
            status = 429,
            body = crate::http::response::ErrorResponse,
            headers(("Retry-After" = u64, description = "建议等待秒数"))
        ),
        (status = 404, body = crate::http::response::ErrorResponse),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
/// 将请求新闻集合渲染为 RSS 2.0 XML
pub(super) async fn rss(
    State(state): State<AppState>,
    ConnectInfo(client_address): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(path): Path<GamePath>,
    MultiQuery(filter_query): MultiQuery<NewsFilterQuery>,
    Query(rss_query): Query<NewsRssQuery>,
) -> Result<Response, AppError> {
    // RSS 查询和签名解析共用一次安全解析的客户端限流检查
    let client_ip = state
        .public_rate_limiters()
        .client_ip(&headers, client_address);
    state.public_rate_limiters().check_rss(client_ip)?;

    let filter = rss_query.apply(filter_query.into_filter(path)?);
    let source_id = filter.filter.source_id.clone();
    let game_id = filter.filter.game_id.clone();
    require_news_source(&state, &game_id, &source_id).await?;

    let result = state.application().list_news_feed(filter).await?;
    let mut items = result.items;

    // 缓存命中不消耗预算，冷缓存最多刷新配置数量的米游社视频签名
    let mut remaining_refreshes = state.config().public_rate_limits.rss_mys_refresh_limit;
    let mut omitted_video_count = 0_u64;
    for item in &mut items {
        if item.source_id == "mys" && item.news_type == "video" {
            item.video_url = match state
                .mys_video_service()
                .resolve_video_url_with_refresh_budget(&game_id, &item.id, &mut remaining_refreshes)
                .await?
            {
                MysVideoUrlResolution::Available(url) => Some(url),
                MysVideoUrlResolution::NotFound => None,
                MysVideoUrlResolution::RefreshBudgetExhausted => {
                    omitted_video_count += 1;
                    None
                }
            };
        }
    }
    if omitted_video_count > 0 {
        tracing::debug!(
            game_id = %game_id,
            source_id = %source_id,
            omitted_video_count,
            refresh_limit = state.config().public_rate_limits.rss_mys_refresh_limit,
            "RSS video signature refresh budget exhausted"
        );
    }
    let document = rss::build(
        &game_id,
        &source_id,
        items,
        result.game_cover,
        &state.config().asset_base_url,
    );

    Ok((
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        document,
    )
        .into_response())
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
