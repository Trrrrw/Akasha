use axum::{Json, extract::Query, http::StatusCode};
use db::entities::{news_items, news_search};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::items::{
    MAX_PAGE_SIZE, NewsErrorResponse, NewsItemSummary, internal_error, load_categories, load_tags,
    to_summary,
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
    let keys = news_search::search(news_search::SearchQuery {
        q: &query.q,
        game: None,
        source: None,
        is_video: query.is_video,
    })
    .await
    .map_err(internal_error)?;

    let mut items = Vec::new();

    for key in keys {
        if let Some(item) =
            news_items::Entity::find_by_id((key.remote_id, key.game_code, key.source))
                .one(db::pool())
                .await
                .map_err(internal_error)?
        {
            let categories = load_categories(&item).await.map_err(internal_error)?;
            let tags = load_tags(&item).await.map_err(internal_error)?;
            items.push(to_summary(item, categories, tags));
        }
    }

    let total = items.len() as u64;
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(items.len());
    let data = if start < items.len() {
        items[start..end].to_vec()
    } else {
        Vec::new()
    };

    Ok(Json(NewsSearchResponse {
        q: query.q,
        page,
        page_size,
        total,
        items: data,
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
