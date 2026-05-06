use axum::{
    Json,
    extract::Query,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use db::entities::{news_items, news_search};
use rss::{ChannelBuilder, GuidBuilder, ItemBuilder};
use sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;
use utoipa::IntoParams;

use super::items::{
    MAX_PAGE_SIZE, NewsErrorResponse, NewsItemKey, NewsItemListRow, internal_error,
    load_categories_for_items, load_list_rows_by_keys, load_tags_for_items,
};

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
    let items = load_news_for_rss(&query, limit)
        .await
        .map_err(internal_error)?;
    let categories = load_categories_for_items(&items)
        .await
        .map_err(internal_error)?;
    let tags = load_tags_for_items(&items).await.map_err(internal_error)?;
    let mut rss_items = Vec::new();

    for item in items {
        let key = item.key();
        let item_categories = categories.get(&key).cloned().unwrap_or_default();
        let item_tags = tags.get(&key).cloned().unwrap_or_default();
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

        item_builder.description(rss_description(
            item.intro.as_deref(),
            &item.cover,
            item.video_url.as_deref(),
            item.is_video,
        ));

        for category in item_categories.into_iter().chain(item_tags) {
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

async fn load_news_for_rss(
    query: &NewsRssQuery,
    limit: u64,
) -> Result<Vec<NewsItemListRow>, sea_orm::DbErr> {
    if let Some(q) = &query.q
        && !q.trim().is_empty()
    {
        return load_news_for_rss_search(q, query.is_video, limit).await;
    }

    let conn = db::pool();
    let mut statement = news_items::Entity::find();

    if let Some(is_video) = query.is_video {
        statement = statement.filter(news_items::Column::IsVideo.eq(is_video));
    }

    statement
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
        .limit(limit)
        .into_model::<NewsItemListRow>()
        .all(conn)
        .await
}

async fn load_news_for_rss_search(
    q: &str,
    is_video: Option<bool>,
    limit: u64,
) -> Result<Vec<NewsItemListRow>, sea_orm::DbErr> {
    let keys = news_search::search(news_search::SearchQuery {
        q,
        game: None,
        source: None,
        is_video,
        limit: Some(limit),
        offset: None,
    })
    .await?;

    let item_keys = keys.iter().map(NewsItemKey::from).collect::<Vec<_>>();

    load_list_rows_by_keys(&item_keys).await
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

fn rss_description(
    value: Option<&str>,
    cover: &str,
    video_url: Option<&str>,
    is_video: bool,
) -> Option<String> {
    let value = value.unwrap_or_default();
    let intro = value.replace('\n', "<br />");

    if is_video {
        return join_rss_parts([cover_image(cover), video_block(video_url), non_empty(intro)]);
    }

    if cover.trim().is_empty() || starts_with_image(value) {
        return non_empty(intro);
    }

    join_rss_parts([cover_image(cover), non_empty(intro)])
}

fn join_rss_parts(parts: impl IntoIterator<Item = Option<String>>) -> Option<String> {
    let parts = parts.into_iter().flatten().collect::<Vec<_>>();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("<br />"))
    }
}

fn starts_with_image(value: &str) -> bool {
    value.trim_start().to_lowercase().starts_with("<img")
}

fn cover_image(cover: &str) -> Option<String> {
    if cover.trim().is_empty() {
        None
    } else {
        Some(format!(r#"<img src="{}">"#, html_attr(cover)))
    }
}

fn video_block(video_url: Option<&str>) -> Option<String> {
    let video_url = video_url?.trim();

    if video_url.is_empty() {
        return None;
    }

    let video_url = html_attr(video_url);

    Some(format!(
        r#"<video controls src="{video_url}"></video><br /><a href="{video_url}">查看视频</a>"#
    ))
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn html_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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
