mod game;
mod http;

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

use chrono::{DateTime, FixedOffset, Utc};
use crawler_core::{
    CrawlerContext, CrawlerTask, Game,
    entities::{games, news_categories, news_categories_link, news_items, news_sources},
    http as core_http, info, warn,
};
use game::MiyousheGameExt;
use serde::{Deserialize, Serialize};

pub struct NewsMiyousheTask;

#[async_trait]
impl CrawlerTask for NewsMiyousheTask {
    fn name(&self) -> &'static str {
        "miyoushe"
    }
    fn display_name(&self) -> &'static str {
        "米游社"
    }
    fn description(&self) -> &'static str {
        "米游社新闻与公告"
    }

    async fn run(&self, _ctx: &CrawlerContext) -> Result<()> {
        news_sources::Entity::create_if_not_exists(news_sources::Model {
            name: self.name().to_string(),
            display_name: self.display_name().to_string(),
            description: self.description().to_string(),
        })
        .await?;

        let mut jobs = tokio::task::JoinSet::new();
        for game in Game::ALL {
            let source = self.name();
            jobs.spawn(async move {
                let result = crawl_game(source, game).await;
                (game, result)
            });
        }
        let mut failures = Vec::new();
        while let Some(result) = jobs.join_next().await {
            match result {
                Ok((_, Ok(()))) => {}
                Ok((game, Err(err))) => {
                    failures.push(format!("{game}: {err:#}"));
                }
                Err(err) => {
                    failures.push(format!("miyoushe join failed: {err}"));
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            anyhow::bail!("miyoushe failed:\n{}", failures.join("\n"));
        }
    }
}

async fn crawl_game(source: &'static str, game: Game) -> Result<()> {
    games::Entity::create_if_not_exists(games::Model {
        game_code: game.to_string(),
        name_en: game.name_en().to_string(),
        name_zh: game.name_zh().to_string(),
        index: game.index() as u32,
        cover: game.default_cover().to_string(),
        extra: None,
    })
    .await?;

    for (index, category) in game.categories().iter().enumerate() {
        news_categories::Entity::create_if_not_exists(news_categories::Model {
            title: category.to_string(),
            game_code: game.to_string(),
            index: index as i32,
        })
        .await?;
    }
    news_categories::Entity::create_if_not_exists(news_categories::Model {
        title: "其他".to_string(),
        game_code: game.to_string(),
        index: i32::MAX,
    })
    .await?;
    info!("{} -> 已初始化米游社分类", game.name_zh());

    let page_size: u32 = 20;
    for news_type in 1..4 {
        let mut last_id: u32 = page_size;
        let mut found_existing = false;
        loop {
            info!(
                "{} -> 正在爬取米游社 type={news_type} page={} last_id={last_id} page_size={page_size}",
                game.name_zh(),
                last_id / page_size
            );

            let params = game.api_params(&last_id, &page_size, &news_type);
            let single_page_data: MiyousheResponse =
                match core_http::get(Game::API_BASE, &params).await {
                    Ok(Some(data)) => data,
                    Ok(None) => {
                        warn!(
                            "{game} -> 获取第 {} 页数据失败，返回空数据，重试中...",
                            last_id / page_size
                        );
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        continue;
                    }
                    Err(err) => {
                        warn!(
                            "{game} -> 获取第 {} 页数据失败，重试中...{err:#}",
                            last_id / page_size
                        );
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        continue;
                    }
                };
            if single_page_data.data.list.is_empty() {
                break;
            }
            info!(
                "{} -> 成功获取米游社 type={news_type} page={}，{} 条",
                game.name_zh(),
                last_id / page_size,
                single_page_data.data.list.len()
            );

            for post in single_page_data.data.list {
                let news_remote_id = post.post.remote_id.clone();
                if news_items::Entity::get_by_pk(&news_remote_id, &game.to_string(), source)
                    .await?
                    .is_some()
                {
                    info!(
                        "{} -> 已找到本地已有文章:《{}》",
                        game.name_zh(),
                        post.post.title,
                    );
                    found_existing = true;
                    break;
                }
                parse_and_save(&post.post, game, source).await?;
            }
            if found_existing {
                break;
            }

            if single_page_data.data.last_id.is_empty() {
                info!("{} -> 米游社 type={news_type} 已无下一页", game.name_zh());
                break;
            }
            last_id = single_page_data.data.last_id.parse().unwrap();
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    Ok(())
}

async fn parse_and_save(post: &MiyoushePost, game: Game, source: &'static str) -> Result<()> {
    let post_data: PostResponse = http::get(game.post_api_url(&post.remote_id)).await?;
    let Some(post_data_detail) = post_data.data.as_ref() else {
        anyhow::bail!(
            "米游社详情接口返回 retcode={} message=\"{}\" post_id={}",
            post_data.retcode,
            post_data.message,
            post.remote_id
        );
    };
    let detail = &post_data_detail.post;
    let detail_post = &detail.post;
    let remote_id = detail_post.remote_id.parse::<i32>().map_err(|err| {
        anyhow::anyhow!("invalid miyoushe post_id {}: {err}", detail_post.remote_id)
    })?;
    let content = &detail_post.content;
    let video_url = extract_video_url(&detail.vod_list);
    let is_video =
        video_url.is_some() || !detail.vod_list.is_empty() || content.contains("mhy-vod");

    let n = news_items::Model {
        remote_id: detail_post.remote_id.clone(),
        game_code: game.to_string(),
        source: source.to_string(),
        title: detail_post.title.clone(),
        intro: extract_intro(content),
        publish_time: detail_post.created_at,
        source_url: game.news_url(&remote_id),
        cover: extract_cover(game.default_cover(), detail),
        is_video,
        video_url,
        raw_data: serde_json::to_string(&post_data)?,
    };

    let news = news_items::Entity::create_if_not_exists(n).await?;

    let categories = game.extract_categories(&detail_post.title, None);
    for category in categories {
        news_categories_link::Entity::create_if_not_exists(news_categories_link::Model {
            news_remote_id: news.remote_id.clone(),
            news_game_belong: news.game_code.clone(),
            news_source_belong: news.source.clone(),
            category_title: category,
            category_game_belong: game.to_string(),
        })
        .await?;
    }

    info!(
        "{} -> 已保存米游社帖子:《{}》",
        game.name_zh(),
        detail_post.title
    );

    Ok(())
}

fn extract_cover(default_cover: &'static str, detail: &Post) -> String {
    detail
        .cover
        .as_ref()
        .map(|cover| cover.url.clone())
        .filter(|cover| !cover.is_empty())
        .or_else(|| {
            detail
                .vod_list
                .first()
                .map(|vod| vod.cover.clone())
                .filter(|cover| !cover.is_empty())
        })
        .or_else(|| (!detail.post.cover.is_empty()).then_some(detail.post.cover.clone()))
        .or_else(|| detail.image_list.first().map(|image| image.url.clone()))
        .unwrap_or_else(|| default_cover.to_string())
}

fn extract_video_url(vod_list: &[Vod]) -> Option<String> {
    vod_list
        .first()
        .and_then(|vod| {
            vod.resolutions
                .iter()
                .max_by_key(|resolution| resolution.height)
        })
        .map(|resolution| resolution.url.clone())
        .filter(|url| !url.is_empty())
}

fn extract_intro(content: &str) -> Option<String> {
    let cleaned = clean_html_content(content);
    let lines: Vec<&str> = cleaned.lines().collect();
    let mut result = Vec::new();
    let mut last_empty = false;

    for line in lines {
        let trimmed = line.trim();
        let is_empty = trimmed.is_empty();

        if is_empty {
            if !last_empty {
                result.push(""); // 保留一个空行
                last_empty = true;
            }
        } else {
            result.push(trimmed);
            last_empty = false;
        }
    }

    let text = result.join("\n");
    (!text.is_empty()).then_some(text)
}

fn clean_html_content(content: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    let mut tag = String::new();

    for ch in content.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' => {
                let tag_name = tag
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .trim_start_matches('/');
                if tag_name == "img" {
                    if let Some(src) = extract_attr(&tag, "src") {
                        text.push_str(&format!(r#"<img src="{src}">"#));
                    }
                } else if matches!(tag_name, "p" | "br" | "div") {
                    text.push('\n');
                }
                in_tag = false;
            }
            _ if in_tag => tag.push(ch),
            _ => text.push(ch),
        }
    }

    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn extract_attr(tag: &str, name: &str) -> Option<String> {
    let pattern = format!(r#"{name}=""#);
    let start = tag.find(&pattern)? + pattern.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

// post list
#[derive(Serialize, Deserialize)]
struct MiyousheResponse {
    data: MiyousheData,
}

#[derive(Serialize, Deserialize)]
struct MiyousheData {
    list: Vec<MiyousheList>,
    last_id: String,
}

#[derive(Serialize, Deserialize)]
struct MiyousheList {
    post: MiyoushePost,
}

#[derive(Serialize, Deserialize)]
struct MiyoushePost {
    #[serde(alias = "post_id")]
    remote_id: String,
    #[serde(alias = "subject")]
    title: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    cover: String,
    #[serde(deserialize_with = "deserialize_unix_seconds")]
    created_at: DateTime<FixedOffset>,
    #[serde(default)]
    images: Vec<String>,
    view_type: i32,
}

fn deserialize_unix_seconds<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let ts = i64::deserialize(deserializer)?;
    let utc = DateTime::<Utc>::from_timestamp(ts, 0)
        .ok_or_else(|| serde::de::Error::custom("invalid unix timestamp"))?;
    let offset = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| serde::de::Error::custom("invalid timezone offset"))?;

    Ok(utc.with_timezone(&offset))
}

// post detail
#[derive(Serialize, Deserialize)]
struct PostResponse {
    data: Option<PostData>,
    retcode: i32,
    #[serde(default)]
    message: String,
}

#[derive(Serialize, Deserialize)]
struct PostData {
    post: Post,
}

#[derive(Serialize, Deserialize)]
struct Post {
    post: PostDetail,
    cover: Option<Image>,
    #[serde(default)]
    image_list: Vec<Image>,
    #[serde(default)]
    vod_list: Vec<Vod>,
}

#[derive(Serialize, Deserialize)]
struct PostDetail {
    #[serde(alias = "post_id")]
    remote_id: String,
    #[serde(alias = "subject")]
    title: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    cover: String,
    #[serde(default)]
    structured_content: String,
    #[serde(deserialize_with = "deserialize_unix_seconds")]
    created_at: DateTime<FixedOffset>,
}

#[derive(Serialize, Deserialize)]
struct Image {
    url: String,
}

#[derive(Serialize, Deserialize)]
struct Vod {
    #[serde(default)]
    cover: String,
    #[serde(default)]
    resolutions: Vec<VodResolution>,
}

#[derive(Serialize, Deserialize)]
struct VodResolution {
    url: String,
    height: i32,
}
