mod game;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};
use scraper::{ElementRef, Html, Node, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

use crawler_core::{
    CrawlerContext, CrawlerTask, Game,
    entities::{games, news_categories, news_categories_link, news_items, news_sources},
    http, info, warn,
};
use game::OfficialSiteGameExt;

pub struct NewsOfficialSiteTask;

#[async_trait]
impl CrawlerTask for NewsOfficialSiteTask {
    fn name(&self) -> &'static str {
        "official_site"
    }
    fn display_name(&self) -> &'static str {
        "官网"
    }
    fn description(&self) -> &'static str {
        "米哈游官网新闻与公告"
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
                    failures.push(format!("official_site join failed: {err}"));
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            anyhow::bail!("official_site failed:\n{}", failures.join("\n"));
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

    let local_latest_news_remote_id =
        match news_items::Entity::get_local_latest_news(Some(source), Some(game.to_string())).await
        {
            Ok(Some(n)) => n.remote_id,
            _ => "".to_string(),
        };

    let mut page_total = 0;
    let mut page_num = 1;
    let mut found_existing = false;
    while page_total == 0 || page_num <= page_total {
        info!(
            "{} -> 正在爬取第 {page_num}/{page_total} 页",
            game.name_zh()
        );

        let single_page_data: OfficialSiteResponse =
            match http::get(game.api_base(), &game.api_params(page_num)).await {
                Ok(Some(data)) => data,
                _ => {
                    warn!("{game} -> 获取第 {page_num} 页数据失败，重试中...");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }
            };
        page_total = match page_total {
            0 => single_page_data.data.total / game.page_size() + 1,
            _ => page_total,
        };
        for news in single_page_data.data.list {
            let news_remote_id = news.id.to_string();
            if news_remote_id == local_latest_news_remote_id {
                info!(
                    "{} -> 已找到本地最新文章:《{}》",
                    game.name_zh(),
                    news.title
                );
                found_existing = true;
                break;
            }
            parse_and_save(news, game, source).await?;
        }
        if found_existing {
            break;
        }
        page_num += 1;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(())
}

async fn parse_and_save(
    raw_data: OfficialSiteItem,
    game: Game,
    source: &'static str,
) -> Result<()> {
    let video_url = extract_video_url(&raw_data.content);
    let title = raw_data.title.clone();

    let n = news_items::Model {
        remote_id: raw_data.id.to_string(),
        game_code: game.to_string(),
        source: source.to_string(),
        title: title.clone(),
        intro: extract_intro(&raw_data.content),
        publish_time: raw_data.publish_time,
        source_url: game.news_url(&raw_data.id),
        cover: extract_cover(game.default_cover(), &raw_data.ext, &raw_data.content),
        is_video: video_url.is_some(),
        video_url: video_url,
        raw_data: serde_json::to_string(&raw_data).expect("serde raw_data err"),
    };
    let news = news_items::Entity::create_if_not_exists(n).await?;

    let categories = game.extract_categories(&title, None);
    for category in categories {
        news_categories_link::Entity::create_if_not_exists(news_categories_link::Model {
            news_remote_id: news.remote_id.clone(),
            news_game_belong: news.game_code.clone(),
            news_source_belong: news.source.clone(),
            category_title: category.to_string(),
            category_game_belong: game.to_string(),
        })
        .await?;
    }

    Ok(())
}

fn extract_intro(content: &str) -> Option<String> {
    let document = Html::parse_document(content);
    let selector = Selector::parse("p").ok()?;

    let mut texts = Vec::new();

    for p in document.select(&selector) {
        let children: Vec<_> = p.children().collect();

        if children.is_empty() {
            texts.push("\n".to_string());
            continue;
        }

        for child in children {
            match child.value() {
                Node::Text(text) => {
                    texts.push(text.text.to_string());
                    texts.push("\n".to_string());
                }
                Node::Element(element) if element.name() == "br" => {
                    texts.push("\n".to_string());
                }
                Node::Element(_) => {
                    if let Some(element) = ElementRef::wrap(child) {
                        texts.push(element.text().collect::<String>());
                    }
                }
                _ => {}
            }
        }
    }

    let intro = texts
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<String>()
        .trim()
        .to_string();

    Some(intro)
}

fn extract_cover(default_cover: &'static str, ext: &str, content: &str) -> String {
    if ext != "{}" {
        if let Ok(value) = serde_json::from_str::<Value>(ext) {
            let ext_cover_keys = ["720_1", "721_1", "news-poster"];

            for key in ext_cover_keys {
                let Some(url) = value
                    .get(key)
                    .and_then(|items| items.as_array())
                    .and_then(|items| items.first())
                    .and_then(|item| item.get("url"))
                    .and_then(|url| url.as_str())
                else {
                    continue;
                };

                return url.to_string();
            }
        }

        return select_first_attr(content, "div video", "poster")
            .unwrap_or_else(|| default_cover.to_string());
    }

    let selectors = ["p video", "p span video", "video"];

    for selector in selectors {
        if let Some(cover) = select_first_attr(content, selector, "poster") {
            return cover;
        }
    }

    default_cover.to_string()
}

fn extract_video_url(content: &str) -> Option<String> {
    let selectors = ["p video", "p span video", "div video"];

    for selector in selectors {
        if let Some(src) = select_first_attr(content, selector, "src") {
            if !src.is_empty() {
                return Some(src);
            }
        }
    }

    None
}

fn select_first_attr(content: &str, selector: &str, attr: &str) -> Option<String> {
    let document = Html::parse_document(content);
    let selector = Selector::parse(selector).ok()?;

    document
        .select(&selector)
        .find_map(|node| node.value().attr(attr).map(ToOwned::to_owned))
}

// DTO
#[derive(Serialize, Deserialize)]
struct OfficialSiteResponse {
    pub data: OfficialSiteData,
}

#[derive(Serialize, Deserialize)]
struct OfficialSiteData {
    #[serde(alias = "iTotal")]
    total: i32,
    list: Vec<OfficialSiteItem>,
}

#[derive(Serialize, Deserialize)]
struct OfficialSiteItem {
    #[serde(alias = "iInfoId")]
    id: i32,
    #[serde(alias = "sTitle")]
    title: String,
    #[serde(
        alias = "dtStartTime",
        deserialize_with = "deserialize_mihoyo_datetime"
    )]
    publish_time: DateTime<FixedOffset>,
    #[serde(alias = "sContent")]
    content: String,
    #[serde(alias = "sExt")]
    ext: String,
}

fn deserialize_mihoyo_datetime<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;

    let naive = NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S")
        .map_err(serde::de::Error::custom)?;

    let offset = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| serde::de::Error::custom("invalid timezone offset"))?;

    offset
        .from_local_datetime(&naive)
        .single()
        .ok_or_else(|| serde::de::Error::custom("invalid datetime"))
}
