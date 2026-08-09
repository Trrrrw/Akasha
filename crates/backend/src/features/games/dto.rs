use akasha_application::games::GameSummary;
use serde::Serialize;
use utoipa::ToSchema;

use crate::features::news::dto::{NewsCount, NewsItemResponse, RecentNews};
use crate::http::response::public_asset_url;

/// 公开游戏列表中的游戏资料
#[derive(Serialize, ToSchema)]
#[schema(description = "游戏信息及新闻摘要")]
pub(super) struct GameResponse {
    /// 游戏 ID
    id: String,
    /// 游戏中文名称
    name: String,
    /// 展示顺序
    index: i64,
    /// 游戏封面绝对地址
    cover: Option<String>,
    /// 游戏图标绝对地址
    icon: Option<String>,
    /// 新闻数量统计
    news_count: NewsCount,
    /// 最近发布的新闻
    recent_news: RecentNews,
}

impl GameResponse {
    /// 将应用层游戏摘要转换为公开游戏响应
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
