use akasha_db::repositories::games::GameSummary;
use serde::Serialize;
use utoipa::ToSchema;

use crate::features::news::dto::{NewsCount, NewsItemResponse, RecentNews};
use crate::http::response::public_asset_url;

#[derive(Serialize, ToSchema)]
#[schema(description = "游戏基础信息")]
pub(super) struct GameResponse {
    id: String,
    name: String,
    index: i64,
    cover: Option<String>,
    icon: Option<String>,
    news_count: NewsCount,
    recent_news: RecentNews,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "游戏详细信息")]
pub(super) struct GameDetailResponse {
    summary: GameResponse,
}

impl GameResponse {
    pub(super) fn from_summary(value: GameSummary, asset_base_url: &str) -> Self {
        let game_cover = value.cover.clone();

        Self {
            id: value.id,
            name: value.name_zh,
            index: value.index,
            cover: public_asset_url(asset_base_url, value.cover),
            icon: public_asset_url(asset_base_url, value.icon),
            news_count: NewsCount {
                total: value.news_count.total,
                article: value.news_count.article,
                video: value.news_count.video,
            },
            recent_news: RecentNews {
                article: value
                    .recent_news
                    .article
                    .into_iter()
                    .map(|news| {
                        NewsItemResponse::from_summary(news, game_cover.as_deref(), asset_base_url)
                    })
                    .collect(),
                video: value
                    .recent_news
                    .video
                    .into_iter()
                    .map(|news| {
                        NewsItemResponse::from_summary(news, game_cover.as_deref(), asset_base_url)
                    })
                    .collect(),
            },
        }
    }
}

impl GameDetailResponse {
    pub(super) fn from_summary(value: GameSummary, asset_base_url: &str) -> Self {
        Self {
            summary: GameResponse::from_summary(value, asset_base_url),
        }
    }
}
