use axum::{
    Json,
    extract::{Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
};

use crate::{
    features::news::{
        dto::{NewsItemResponse, NewsListMeta, NewsSourceResponse, NewsTagsResponse},
        query::{NewsDetailPath, NewsDetailQuery, NewsListQuery, NewsTagsQuery},
        rss, use_cases,
    },
    http::{
        error::AppError,
        path::GamePath,
        response::{ListResponse, PageResponse},
    },
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/games/{game_id}/news/sources",
    tag = "News",
    summary = "获取新闻来源列表",
    params(GamePath),
    responses(
        (status = 200, body = ListResponse<NewsSourceResponse>),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
pub(super) async fn list_sources(
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
) -> Result<Json<ListResponse<NewsSourceResponse>>, AppError> {
    let rows = use_cases::list_sources(state.db(), &game_id)
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    let items = rows
        .into_iter()
        .map(NewsSourceResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(ListResponse {
        total: items.len() as u64,
        items,
    }))
}

#[utoipa::path(get, path = "/games/{game_id}/news/tags",
tag = "News",
summary = "获取新闻标签列表",
params(GamePath, NewsTagsQuery),
responses(
    (status = 200, body = NewsTagsResponse),
    (status = 500, body = crate::http::response::ErrorResponse)
))]
pub(super) async fn list_tags(
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
    Query(NewsTagsQuery { source_id }): Query<NewsTagsQuery>,
) -> Result<Json<NewsTagsResponse>, AppError> {
    let rows = use_cases::list_tags(state.db(), &game_id, &source_id)
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    Ok(Json(NewsTagsResponse::from_rows(game_id, source_id, rows)))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/news",
    tag = "News",
    summary = "获取新闻列表",
    params(GamePath, NewsListQuery),
    responses(
        (status = 200, body = PageResponse<NewsItemResponse, NewsListMeta>),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
pub(super) async fn list(
    State(state): State<AppState>,
    Path(path): Path<GamePath>,
    Query(query): Query<NewsListQuery>,
) -> Result<Json<PageResponse<NewsItemResponse, NewsListMeta>>, AppError> {
    let filter = query.into_filter(path)?;
    let source_id = filter.source_id.clone();
    let game_id = filter.game_id.clone();
    let limit = filter.limit;
    let offset = filter.offset;

    tracing::debug!(
        game_id = %game_id,
        source_id = %source_id,
        limit,
        offset,
        "listing news"
    );

    let (total, rows, game_cover) = use_cases::list(state.db(), filter)
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    let items = rows
        .into_iter()
        .map(|news| NewsItemResponse::from_summary(news, game_cover.as_deref()))
        .collect();

    Ok(Json(PageResponse {
        total,
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
    params(NewsDetailPath, NewsDetailQuery),
    responses(
        (status = 200, body = NewsItemResponse),
        (status = 404, body = crate::http::response::ErrorResponse),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
pub(super) async fn detail(
    State(state): State<AppState>,
    Path(NewsDetailPath { game_id, news_id }): Path<NewsDetailPath>,
    Query(NewsDetailQuery { source_id }): Query<NewsDetailQuery>,
) -> Result<Json<NewsItemResponse>, AppError> {
    let (news, game_cover) = use_cases::detail(state.db(), &game_id, &source_id, &news_id)
        .await
        .map_err(|error| AppError::Internal(error.into()))?
        .ok_or_else(|| {
            AppError::NotFound(format!("news {news_id} not found in {source_id} {game_id}"))
        })?;

    Ok(Json(NewsItemResponse::from_summary(
        news,
        game_cover.as_deref(),
    )))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/news/rss",
    tag = "News",
    summary = "获取新闻 RSS",
    params(GamePath, NewsListQuery),
    responses(
        (status = 200, description = "RSS 2.0 XML"),
        (status = 500, body = crate::http::response::ErrorResponse)
    )
)]
pub(super) async fn rss(
    State(state): State<AppState>,
    Path(path): Path<GamePath>,
    Query(query): Query<NewsListQuery>,
) -> Result<Response, AppError> {
    let filter = query.into_filter(path)?;
    let source_id = filter.source_id.clone();
    let game_id = filter.game_id.clone();

    let (_, rows, game_cover) = use_cases::list(state.db(), filter)
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    let document = rss::build(&game_id, &source_id, rows, game_cover);

    Ok((
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        document,
    )
        .into_response())
}
