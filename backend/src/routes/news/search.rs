use axum::{Json, extract::Query, http::StatusCode};
use db::entities::news_search;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::items::{
    MAX_PAGE_SIZE, NewsErrorResponse, NewsItemKey, NewsItemSummary, internal_error,
    load_categories_for_items, load_list_rows_by_keys, load_tags_for_items,
};
use axum_mcp::{MCPInputSchema, mcp};

const DEFAULT_PAGE: u64 = 1;
const DEFAULT_PAGE_SIZE: u64 = 20;

#[mcp]
#[utoipa::path(
    get,
    path = "/search",
    tag = "News",
    summary = "搜索新闻",
    description = "根据搜索语法搜索新闻，支持空格 AND、竖线 OR、减号排除、引号短语，以及 title:、tag:、game:、source:、category: 字段限定。",
    params(NewsSearchQuery),
    responses(
        (status = 200, body = NewsSearchResponse),
        (status = 500, body = NewsErrorResponse)
    )
)]
pub async fn news_search(
    Query(query): Query<NewsSearchQuery>,
) -> Result<Json<NewsSearchResponse>, (StatusCode, Json<NewsErrorResponse>)> {
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = (page - 1) * page_size;
    let total = news_search::count(news_search::SearchQuery {
        q: &query.q,
        game: None,
        source: None,
        is_video: query.is_video,
        limit: None,
        offset: None,
    })
    .await
    .map_err(internal_error)?;
    let keys = news_search::search(news_search::SearchQuery {
        q: &query.q,
        game: None,
        source: None,
        is_video: query.is_video,
        limit: Some(page_size),
        offset: Some(offset),
    })
    .await
    .map_err(internal_error)?;
    let item_keys = keys.iter().map(NewsItemKey::from).collect::<Vec<_>>();
    let rows = load_list_rows_by_keys(&item_keys)
        .await
        .map_err(internal_error)?;
    let categories = load_categories_for_items(&rows)
        .await
        .map_err(internal_error)?;
    let tags = load_tags_for_items(&rows).await.map_err(internal_error)?;
    let items = rows
        .into_iter()
        .map(|item| {
            let key = item.key();
            let categories = categories.get(&key).cloned().unwrap_or_default();
            let tags = tags.get(&key).cloned().unwrap_or_default();
            item.to_summary(categories, tags)
        })
        .collect();

    Ok(Json(NewsSearchResponse {
        q: query.q,
        page,
        page_size,
        total,
        items,
    }))
}

#[derive(Deserialize, IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Query)]
pub struct NewsSearchQuery {
    /// 搜索语法。game: 可传游戏代码或中英文名称；示例：game:原神 source:official_site 版本|前瞻 -修复 title:调频 tag:角色 category:公告。
    q: String,
    /// 按新闻视频类型筛选；true 表示只返回视频新闻，false 表示只返回非视频新闻。
    is_video: Option<bool>,
    /// 页码，从 1 开始。
    page: Option<u64>,
    /// 每页数量，最大 100。
    page_size: Option<u64>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻搜索响应。")]
pub struct NewsSearchResponse {
    /// 搜索关键词。
    q: String,
    /// 当前页码，从 1 开始。
    page: u64,
    /// 每页数量。
    page_size: u64,
    /// 符合搜索和筛选条件的新闻总数。
    total: u64,
    /// 当前页新闻列表。
    items: Vec<NewsItemSummary>,
}
