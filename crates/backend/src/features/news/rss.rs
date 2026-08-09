use akasha_application::news::NewsSummary;
use rss::{ChannelBuilder, GuidBuilder, ItemBuilder};

use crate::http::response::public_asset_url;

/// 根据应用层新闻读取模型构建 RSS 2.0 文档
pub(super) fn build(
    game_id: &str,
    source_id: &str,
    rows: Vec<NewsSummary>,
    game_cover: Option<String>,
    asset_base_url: &str,
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
                    public_asset_url(asset_base_url, news.cover.or_else(|| game_cover.clone())),
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
        .link(asset_base_url)
        .description("米哈游游戏信息聚合 API")
        .generator("Trrrrw -- trrw.cn".to_string())
        .language("zh-cn".to_string())
        .ttl(Some("5".to_string()))
        .last_build_date(last_build_date)
        .items(items)
        .build()
        .to_string()
}

/// 为 RSS 阅读器构建安全的 HTML 条目内容
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
        parts.push(format!(r#"<img src="{}">"#, escape_html_attribute(&cover)));
    }

    if news_type == "video"
        && let Some(video_url) = video_url
    {
        parts.push(format!(
            r#"<video controls src="{}"></video>"#,
            escape_html_attribute(&video_url)
        ));
    }

    if !intro.is_empty() {
        parts.push(intro.replace('\n', "<br />"));
    }

    parts.join("<br />")
}

/// 转义嵌入 RSS HTML 属性的外部地址
fn escape_html_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::description;

    /// RSS 媒体地址不能逃逸 HTML 属性
    #[test]
    fn escapes_media_html_attributes() {
        let rendered = description(
            Some("<p>介绍</p>".to_owned()),
            Some("https://example.com/image.jpg?x=1&y=\"bad\"".to_owned()),
            Some("https://example.com/video.mp4?a=1&b='bad'".to_owned()),
            "video".to_owned(),
        );

        assert!(rendered.contains("x=1&amp;y=&quot;bad&quot;"));
        assert!(rendered.contains("a=1&amp;b=&#39;bad&#39;"));
        assert!(!rendered.contains("y=\"bad\""));
    }
}
