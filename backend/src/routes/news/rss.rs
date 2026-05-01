use axum::{
    Json,
    extract::Query,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use db::entities::{news_items, news_search};
use rss::{ChannelBuilder, GuidBuilder, ItemBuilder};
use sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder};
use serde::Deserialize;
use utoipa::IntoParams;

use super::items::{MAX_PAGE_SIZE, NewsErrorResponse, internal_error, load_categories, load_tags};

const DEFAULT_RSS_LIMIT: u64 = 50;

#[utoipa::path(
    get,
    path = "/rss",
    tag = "News",
    summary = "获取新闻 RSS",
    description = "返回最新一批新闻的 RSS 2.0 XML；通过 q 复用新闻搜索语法，可用 is_video 筛选视频类型。",
    params(NewsRssQuery),
    responses(
        (status = 200, body = String, description = "RSS 2.0 XML"),
        (status = 500, body = NewsErrorResponse)
    )
)]
pub async fn news_rss(
    Query(query): Query<NewsRssQuery>,
) -> Result<Response, (StatusCode, Json<NewsErrorResponse>)> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_RSS_LIMIT)
        .clamp(1, MAX_PAGE_SIZE);
    let items = load_news_for_rss(&query).await.map_err(internal_error)?;
    let mut rss_items = Vec::new();

    for item in items.into_iter().take(limit as usize) {
        let categories = load_categories(&item).await.map_err(internal_error)?;
        let tags = load_tags(&item).await.map_err(internal_error)?;
        let mut guid_builder = GuidBuilder::default();
        guid_builder
            .value(format!(
                "{}:{}:{}",
                item.source, item.game_code, item.remote_id
            ))
            .permalink(false);
        let guid = guid_builder.build();

        let mut item_builder = ItemBuilder::default();
        item_builder
            .title(Some(item.title))
            .link(Some(item.source_url))
            .pub_date(Some(item.publish_time.to_rfc2822()))
            .guid(Some(guid));

        if let Some(intro) = item.intro {
            item_builder.description(Some(intro));
        }

        for category in categories.into_iter().chain(tags.into_iter()) {
            item_builder.category(category.into());
        }

        rss_items.push(item_builder.build());
    }

    let mut channel_builder = ChannelBuilder::default();
    channel_builder
        .title(channel_title(&query))
        .link("http://localhost:3000/news/rss")
        .description("Akasha 收录的米哈游游戏新闻订阅。")
        .language(Some("zh-CN".to_string()))
        .ttl(Some("5".to_string()))
        .items(rss_items);
    let channel = channel_builder.build();

    Ok((
        [
            (header::CONTENT_TYPE, "application/rss+xml; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=300"),
        ],
        channel.to_string(),
    )
        .into_response())
}

async fn load_news_for_rss(query: &NewsRssQuery) -> Result<Vec<news_items::Model>, sea_orm::DbErr> {
    if let Some(q) = &query.q
        && !q.trim().is_empty()
    {
        return load_news_for_rss_search(q, query.is_video).await;
    }

    let conn = db::pool();
    let mut statement = news_items::Entity::find();

    if let Some(is_video) = query.is_video {
        statement = statement.filter(news_items::Column::IsVideo.eq(is_video));
    }

    statement
        .order_by(news_items::Column::PublishTime, Order::Desc)
        .all(conn)
        .await
}

async fn load_news_for_rss_search(
    q: &str,
    is_video: Option<bool>,
) -> Result<Vec<news_items::Model>, sea_orm::DbErr> {
    let keys = news_search::search(news_search::SearchQuery {
        q,
        game: None,
        source: None,
        is_video,
    })
    .await?;
    let mut items = Vec::new();

    for key in keys {
        if let Some(item) =
            news_items::Entity::find_by_id((key.remote_id, key.game_code, key.source))
                .one(db::pool())
                .await?
        {
            items.push(item);
        }
    }

    Ok(items)
}

fn channel_title(query: &NewsRssQuery) -> String {
    let mut parts = vec!["Akasha 新闻".to_string()];

    if let Some(q) = &query.q
        && !q.trim().is_empty()
    {
        parts.push(q.clone());
    }

    if query.is_video == Some(true) {
        parts.push("视频".to_string());
    }

    parts.join(" - ")
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct NewsRssQuery {
    /// 搜索语法。game: 可传游戏代码或中英文名称；示例：game:原神 source:official_site 版本|前瞻 -修复 title:调频 tag:角色 category:公告。
    q: Option<String>,
    /// 按新闻视频类型筛选；true 表示只返回视频新闻，false 表示只返回非视频新闻。
    is_video: Option<bool>,
    /// RSS 条目数量，默认 50，最大 100。
    limit: Option<u64>,
}
