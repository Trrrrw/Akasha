use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
};
use chrono::{DateTime, FixedOffset};
use db::entities::{
    games, news_categories, news_categories_link, news_items, news_search, news_tags_link, tags,
};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, FromQueryResult, Order, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};

use axum_mcp::{MCPInputSchema, mcp};

const DEFAULT_PAGE: u64 = 1;
const DEFAULT_PAGE_SIZE: u64 = 20;
pub(super) const MAX_PAGE_SIZE: u64 = 100;

#[mcp]
#[utoipa::path(
    get,
    path = "/items",
    tag = "News",
    summary = "获取新闻列表",
    description = "按游戏、来源、自动分类、人工标签和视频类型筛选新闻，返回分页后的轻量新闻列表。",
    params(NewsItemsQuery),
    responses(
        (status = 200, body = NewsItemsResponse),
        (status = 500, body = NewsErrorResponse)
    )
)]
pub async fn news_items(
    Query(query): Query<NewsItemsQuery>,
) -> Result<Json<NewsItemsResponse>, (StatusCode, Json<NewsErrorResponse>)> {
    let conn = db::pool();
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let mut statement = news_items::Entity::find();

    if let Some(game_code) = resolve_optional_game(query.game.as_deref())
        .await
        .map_err(internal_error)?
    {
        statement = statement.filter(news_items::Column::GameCode.eq(game_code));
    } else if query.game.is_some() {
        return Ok(Json(NewsItemsResponse {
            page,
            page_size,
            total: 0,
            items: Vec::new(),
        }));
    }

    if let Some(source) = &query.source {
        statement = statement.filter(news_items::Column::Source.eq(source));
    }

    if let Some(is_video) = query.is_video {
        statement = statement.filter(news_items::Column::IsVideo.eq(is_video));
    }

    if let Some(category) = &query.category {
        statement = statement.filter(Expr::cust_with_values(
            r#"EXISTS (
                SELECT 1
                FROM news_categories_link AS ncl
                WHERE ncl.news_remote_id = news_items.remote_id
                    AND ncl.news_game_belong = news_items.game_code
                    AND ncl.news_source_belong = news_items.source
                    AND ncl.category_title = ?
            )"#,
            [category.to_owned()],
        ));
    }

    if let Some(tag) = &query.tag {
        statement = statement.filter(Expr::cust_with_values(
            r#"EXISTS (
                SELECT 1
                FROM news_tags_link AS ntl
                WHERE ntl.news_remote_id = news_items.remote_id
                    AND ntl.news_game_belong = news_items.game_code
                    AND ntl.news_source_belong = news_items.source
                    AND ntl.tag_title = ?
            )"#,
            [tag.to_owned()],
        ));
    }

    let total = statement
        .clone()
        .count(conn)
        .await
        .map_err(internal_error)?;
    let offset = (page - 1) * page_size;
    let news = statement
        .select_only()
        .column(news_items::Column::RemoteId)
        .column(news_items::Column::GameCode)
        .column(news_items::Column::Source)
        .column(news_items::Column::Title)
        .column(news_items::Column::Intro)
        .column(news_items::Column::PublishTime)
        .column(news_items::Column::SourceUrl)
        .column(news_items::Column::Cover)
        .column(news_items::Column::IsVideo)
        .column(news_items::Column::VideoUrl)
        .order_by(news_items::Column::PublishTime, Order::Desc)
        .offset(offset)
        .limit(page_size)
        .into_model::<NewsItemListRow>()
        .all(conn)
        .await
        .map_err(internal_error)?;

    let categories = load_categories_for_items(&news)
        .await
        .map_err(internal_error)?;
    let tags = load_tags_for_items(&news).await.map_err(internal_error)?;
    let items = news
        .into_iter()
        .map(|item| {
            let key = item.key();
            let categories = categories.get(&key).cloned().unwrap_or_default();
            let tags = tags.get(&key).cloned().unwrap_or_default();
            item.to_summary(categories, tags)
        })
        .collect();

    Ok(Json(NewsItemsResponse {
        page,
        page_size,
        total,
        items,
    }))
}

#[mcp]
#[utoipa::path(
    get,
    path = "/items/{source}/{game_code}/{remote_id}",
    tag = "News",
    summary = "获取新闻详情",
    description = "根据新闻来源、游戏代码和远端新闻 ID 获取单条新闻的详情元信息。",
    params(NewsItemPath),
    responses(
        (status = 200, body = NewsItemResponse),
        (status = 404, body = NewsErrorResponse),
        (status = 500, body = NewsErrorResponse)
    )
)]
pub async fn news_item(
    Path(path): Path<NewsItemPath>,
) -> Result<Json<NewsItemResponse>, (StatusCode, Json<NewsErrorResponse>)> {
    let item = find_item(&path.source, &path.game_code, &path.remote_id).await?;
    let categories = load_categories(&item).await.map_err(internal_error)?;
    let tags = load_tags(&item).await.map_err(internal_error)?;

    Ok(Json(NewsItemResponse {
        item: to_detail(item, categories, tags),
    }))
}

#[mcp]
#[utoipa::path(
    get,
    path = "/items/{source}/{game_code}/{remote_id}/related",
    tag = "News",
    summary = "获取相关新闻",
    description = "在同一来源和同一游戏内，根据相同自动分类查找相关新闻；可通过 is_video 参数筛选视频或非视频新闻。",
    params(NewsItemPath, RelatedNewsQuery),
    responses(
        (status = 200, body = RelatedNewsResponse),
        (status = 404, body = NewsErrorResponse),
        (status = 500, body = NewsErrorResponse)
    )
)]
pub async fn news_item_related(
    Path(path): Path<NewsItemPath>,
    Query(query): Query<RelatedNewsQuery>,
) -> Result<Json<RelatedNewsResponse>, (StatusCode, Json<NewsErrorResponse>)> {
    let item = find_item(&path.source, &path.game_code, &path.remote_id).await?;
    let categories = load_categories(&item).await.map_err(internal_error)?;
    let limit = query.limit.unwrap_or(10).clamp(1, MAX_PAGE_SIZE);

    let candidates = news_items::Entity::find()
        .filter(news_items::Column::Source.eq(&item.source))
        .filter(news_items::Column::GameCode.eq(&item.game_code));
    let candidates = if let Some(is_video) = query.is_video {
        candidates.filter(news_items::Column::IsVideo.eq(is_video))
    } else {
        candidates
    };
    let candidates = candidates
        .order_by(news_items::Column::PublishTime, Order::Desc)
        .all(db::pool())
        .await
        .map_err(internal_error)?;

    let mut related = Vec::new();

    for candidate in candidates {
        if candidate.remote_id == item.remote_id
            && candidate.game_code == item.game_code
            && candidate.source == item.source
        {
            continue;
        }

        let candidate_categories = load_categories(&candidate).await.map_err(internal_error)?;
        if !categories.is_empty()
            && !candidate_categories
                .iter()
                .any(|category| categories.contains(category))
        {
            continue;
        }

        let candidate_tags = load_tags(&candidate).await.map_err(internal_error)?;
        related.push(to_summary(candidate, candidate_categories, candidate_tags));

        if related.len() >= limit as usize {
            break;
        }
    }

    Ok(Json(RelatedNewsResponse { items: related }))
}

async fn find_item(
    source: &str,
    game_code: &str,
    remote_id: &str,
) -> Result<news_items::Model, (StatusCode, Json<NewsErrorResponse>)> {
    news_items::Entity::find_by_id((
        remote_id.to_string(),
        game_code.to_string(),
        source.to_string(),
    ))
    .one(db::pool())
    .await
    .map_err(internal_error)?
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(NewsErrorResponse {
                message: "news item not found".to_string(),
            }),
        )
    })
}

pub(super) async fn load_categories(
    item: &news_items::Model,
) -> Result<Vec<String>, sea_orm::DbErr> {
    let links = news_categories_link::Entity::find()
        .filter(news_categories_link::Column::NewsRemoteId.eq(&item.remote_id))
        .filter(news_categories_link::Column::NewsGameBelong.eq(&item.game_code))
        .filter(news_categories_link::Column::NewsSourceBelong.eq(&item.source))
        .all(db::pool())
        .await?;

    let mut values = Vec::new();

    for link in links {
        if news_categories::Entity::find_by_id((
            link.category_title.clone(),
            link.category_game_belong.clone(),
        ))
        .one(db::pool())
        .await?
        .is_some()
        {
            values.push(link.category_title);
        }
    }

    Ok(values)
}

pub(super) async fn load_tags(item: &news_items::Model) -> Result<Vec<String>, sea_orm::DbErr> {
    let links = news_tags_link::Entity::find()
        .filter(news_tags_link::Column::NewsRemoteId.eq(&item.remote_id))
        .filter(news_tags_link::Column::NewsGameBelong.eq(&item.game_code))
        .filter(news_tags_link::Column::NewsSourceBelong.eq(&item.source))
        .all(db::pool())
        .await?;

    let mut values = Vec::new();

    for link in links {
        if tags::Entity::find_by_id((link.tag_title.clone(), link.tag_game_belong.clone()))
            .one(db::pool())
            .await?
            .is_some()
        {
            values.push(link.tag_title);
        }
    }

    Ok(values)
}

pub(super) async fn load_categories_for_items(
    items: &[NewsItemListRow],
) -> Result<HashMap<NewsItemKey, Vec<String>>, sea_orm::DbErr> {
    if items.is_empty() {
        return Ok(HashMap::new());
    }

    let links = news_categories_link::Entity::find()
        .filter(link_key_condition(
            items,
            news_categories_link::Column::NewsRemoteId,
            news_categories_link::Column::NewsGameBelong,
            news_categories_link::Column::NewsSourceBelong,
        ))
        .all(db::pool())
        .await?;

    let mut values = HashMap::<NewsItemKey, Vec<String>>::new();
    for link in links {
        values
            .entry(NewsItemKey {
                remote_id: link.news_remote_id,
                game_code: link.news_game_belong,
                source: link.news_source_belong,
            })
            .or_default()
            .push(link.category_title);
    }

    Ok(values)
}

pub(super) async fn load_tags_for_items(
    items: &[NewsItemListRow],
) -> Result<HashMap<NewsItemKey, Vec<String>>, sea_orm::DbErr> {
    if items.is_empty() {
        return Ok(HashMap::new());
    }

    let links = news_tags_link::Entity::find()
        .filter(link_key_condition(
            items,
            news_tags_link::Column::NewsRemoteId,
            news_tags_link::Column::NewsGameBelong,
            news_tags_link::Column::NewsSourceBelong,
        ))
        .all(db::pool())
        .await?;

    let mut values = HashMap::<NewsItemKey, Vec<String>>::new();
    for link in links {
        values
            .entry(NewsItemKey {
                remote_id: link.news_remote_id,
                game_code: link.news_game_belong,
                source: link.news_source_belong,
            })
            .or_default()
            .push(link.tag_title);
    }

    Ok(values)
}

fn link_key_condition<C>(
    items: &[NewsItemListRow],
    remote_id_column: C,
    game_code_column: C,
    source_column: C,
) -> Condition
where
    C: ColumnTrait,
{
    items.iter().fold(Condition::any(), |condition, item| {
        condition.add(
            Condition::all()
                .add(remote_id_column.eq(&item.remote_id))
                .add(game_code_column.eq(&item.game_code))
                .add(source_column.eq(&item.source)),
        )
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct NewsItemKey {
    pub(super) remote_id: String,
    pub(super) game_code: String,
    pub(super) source: String,
}

#[derive(Clone, FromQueryResult)]
pub(super) struct NewsItemListRow {
    pub(super) remote_id: String,
    pub(super) game_code: String,
    pub(super) source: String,
    pub(super) title: String,
    pub(super) intro: Option<String>,
    pub(super) publish_time: DateTime<FixedOffset>,
    pub(super) source_url: String,
    pub(super) cover: String,
    pub(super) is_video: bool,
    pub(super) video_url: Option<String>,
}

impl NewsItemListRow {
    pub(super) fn key(&self) -> NewsItemKey {
        NewsItemKey {
            remote_id: self.remote_id.clone(),
            game_code: self.game_code.clone(),
            source: self.source.clone(),
        }
    }

    pub(super) fn to_summary(&self, categories: Vec<String>, tags: Vec<String>) -> NewsItemSummary {
        NewsItemSummary {
            remote_id: self.remote_id.clone(),
            game_code: self.game_code.clone(),
            source: self.source.clone(),
            title: self.title.clone(),
            intro: self.intro.clone(),
            publish_time: format_datetime(self.publish_time),
            source_url: self.source_url.clone(),
            cover: self.cover.clone(),
            is_video: self.is_video,
            video_url: self.video_url.clone(),
            categories,
            tags,
        }
    }
}

impl From<&news_search::SearchKey> for NewsItemKey {
    fn from(key: &news_search::SearchKey) -> Self {
        Self {
            remote_id: key.remote_id.clone(),
            game_code: key.game_code.clone(),
            source: key.source.clone(),
        }
    }
}

pub(super) async fn load_list_rows_by_keys(
    keys: &[NewsItemKey],
) -> Result<Vec<NewsItemListRow>, sea_orm::DbErr> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let rows = news_items::Entity::find()
        .select_only()
        .column(news_items::Column::RemoteId)
        .column(news_items::Column::GameCode)
        .column(news_items::Column::Source)
        .column(news_items::Column::Title)
        .column(news_items::Column::Intro)
        .column(news_items::Column::PublishTime)
        .column(news_items::Column::SourceUrl)
        .column(news_items::Column::Cover)
        .column(news_items::Column::IsVideo)
        .column(news_items::Column::VideoUrl)
        .filter(keys.iter().fold(Condition::any(), |condition, key| {
            condition.add(
                Condition::all()
                    .add(news_items::Column::RemoteId.eq(&key.remote_id))
                    .add(news_items::Column::GameCode.eq(&key.game_code))
                    .add(news_items::Column::Source.eq(&key.source)),
            )
        }))
        .into_model::<NewsItemListRow>()
        .all(db::pool())
        .await?;
    let mut rows_by_key = rows
        .into_iter()
        .map(|row| (row.key(), row))
        .collect::<HashMap<_, _>>();

    Ok(keys
        .iter()
        .filter_map(|key| rows_by_key.remove(key))
        .collect())
}

pub(super) fn to_summary(
    item: news_items::Model,
    categories: Vec<String>,
    tags: Vec<String>,
) -> NewsItemSummary {
    NewsItemSummary {
        remote_id: item.remote_id,
        game_code: item.game_code,
        source: item.source,
        title: item.title,
        intro: item.intro,
        publish_time: format_datetime(item.publish_time),
        source_url: item.source_url,
        cover: item.cover,
        is_video: item.is_video,
        video_url: item.video_url,
        categories,
        tags,
    }
}

fn to_detail(
    item: news_items::Model,
    categories: Vec<String>,
    tags: Vec<String>,
) -> NewsItemDetail {
    NewsItemDetail {
        summary: to_summary(item, categories, tags),
    }
}

fn format_datetime(value: DateTime<FixedOffset>) -> String {
    let offset = FixedOffset::east_opt(8 * 3600).expect("valid timezone offset");
    value.with_timezone(&offset).to_rfc3339()
}

pub(super) fn internal_error(err: sea_orm::DbErr) -> (StatusCode, Json<NewsErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(NewsErrorResponse {
            message: err.to_string(),
        }),
    )
}

async fn resolve_optional_game(input: Option<&str>) -> Result<Option<String>, sea_orm::DbErr> {
    match input {
        Some(value) => games::Entity::resolve_game_code(value).await,
        None => Ok(None),
    }
}

#[derive(Deserialize, IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Query)]
pub struct NewsItemsQuery {
    /// 游戏代码或中英文名称，例如 hk4e、原神、Genshin Impact。
    game: Option<String>,
    /// 新闻来源，例如 official_site。
    source: Option<String>,
    /// 程序自动分类名称。
    category: Option<String>,
    /// 人工标签名称，以category为主，tag只做辅助用，大部分为空。
    tag: Option<String>,
    /// 按新闻视频类型筛选；true 表示只返回视频新闻，false 表示只返回非视频新闻。
    is_video: Option<bool>,
    /// 页码，从 1 开始。
    page: Option<u64>,
    /// 每页数量，最大 100。
    page_size: Option<u64>,
}

#[derive(Deserialize, IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Path)]
pub struct NewsItemPath {
    /// 新闻来源，例如 official_site。
    source: String,
    /// 游戏代码，例如 hk4e、hkrpg、nap。
    game_code: String,
    /// 新闻在来源站点中的远端 ID。
    remote_id: String,
}

#[derive(Deserialize, IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Query)]
pub struct RelatedNewsQuery {
    /// 返回数量上限，最大 100。
    limit: Option<u64>,
    /// 按新闻视频类型筛选相关新闻；true 表示只返回视频新闻，false 表示只返回非视频新闻。
    is_video: Option<bool>,
}

#[derive(Clone, Serialize, ToSchema)]
#[schema(description = "新闻列表中的单条新闻摘要。")]
pub struct NewsItemSummary {
    /// 新闻在来源站点中的远端 ID。
    remote_id: String,
    /// 游戏代码。
    game_code: String,
    /// 新闻来源。
    source: String,
    /// 新闻标题。
    title: String,
    /// 新闻简介。
    intro: Option<String>,
    /// 新闻发布时间，RFC3339 格式。
    publish_time: String,
    /// 来源站点中的新闻详情页地址。
    source_url: String,
    /// 封面图片地址。
    cover: String,
    /// 是否为视频新闻。
    is_video: bool,
    /// 视频地址。
    video_url: Option<String>,
    /// 程序自动分类。
    categories: Vec<String>,
    /// 人工标签。
    tags: Vec<String>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻详情数据。")]
pub struct NewsItemDetail {
    /// 新闻摘要信息。
    summary: NewsItemSummary,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻列表响应。")]
pub struct NewsItemsResponse {
    /// 当前页码，从 1 开始。
    page: u64,
    /// 每页数量。
    page_size: u64,
    /// 符合筛选条件的新闻总数。
    total: u64,
    /// 当前页新闻列表。
    items: Vec<NewsItemSummary>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻详情响应。")]
pub struct NewsItemResponse {
    /// 新闻详情。
    item: NewsItemDetail,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "相关新闻响应。")]
pub struct RelatedNewsResponse {
    /// 相关新闻列表。
    items: Vec<NewsItemSummary>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻接口错误响应。")]
pub struct NewsErrorResponse {
    /// 错误信息。
    message: String,
}
