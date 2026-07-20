use akasha_db::repositories::news::NewsSummary;
use rss::{ChannelBuilder, GuidBuilder, ItemBuilder};

pub(super) fn build(
    game_id: &str,
    source_id: &str,
    rows: Vec<NewsSummary>,
    game_cover: Option<String>,
) -> String {
    let last_build_date = rows.first().map(|item| item.publish_time.to_rfc2822());

    let items = rows
        .into_iter()
        .map(|news| {
            let mut item_builder = ItemBuilder::default();

            item_builder
                .title(Some(news.title))
                .link(Some(news.source_url.clone()))
                .pub_date(Some(news.publish_time.to_rfc2822()))
                .guid(Some(
                    GuidBuilder::default()
                        .value(format!("{game_id}:{source_id}:{}", news.id))
                        .permalink(false)
                        .build(),
                ))
                .description(description(
                    news.intro,
                    news.cover.or_else(|| game_cover.clone()),
                    news.video_url,
                    news.news_type,
                ));

            for tag in news.tags {
                item_builder.category(tag.into());
            }

            item_builder.build()
        })
        .collect::<Vec<_>>();

    ChannelBuilder::default()
        .title("Akasha News")
        .link("http://akasha.trrw.tech/")
        .description("米哈游游戏信息聚合 API")
        .generator("Trrrrw -- trrw.tech".to_string())
        .language("zh-cn".to_string())
        .ttl(Some("5".to_string()))
        .last_build_date(last_build_date)
        .items(items)
        .build()
        .to_string()
}

fn description(
    intro: Option<String>,
    cover: Option<String>,
    video_url: Option<String>,
    news_type: String,
) -> String {
    let mut parts = Vec::new();
    let intro = intro.unwrap_or_default();

    if !intro.is_empty()
        && !intro.trim().starts_with("<img")
        && let Some(cover) = cover.filter(|cover| !cover.is_empty())
    {
        parts.push(format!(r#"<img src="{cover}">"#));
    }

    if news_type == "video"
        && let Some(video_url) = video_url
    {
        parts.push(format!(r#"<video controls src="{video_url}"></video>"#));
    }

    if !intro.is_empty() {
        parts.push(intro.replace('\n', "<br />"));
    }

    parts.join("<br />")
}
