use akasha_application::games::GameSummary;
use serde::Serialize;
use utoipa::ToSchema;

use crate::features::news::dto::{NewsCount, NewsItemResponse, RecentNews};
use crate::http::response::public_asset_url;

const GAME_ICON_SIZES: [u16; 3] = [64, 128, 256];

/// 指定边长的正方形游戏图标
#[derive(Serialize, ToSchema)]
pub(super) struct GameIconVariantResponse {
    /// 图标宽高，单位为像素
    size: u16,
    /// 图标绝对地址
    url: String,
}

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
    /// 适合不同显示密度的游戏图标
    icon_variants: Vec<GameIconVariantResponse>,
    /// 新闻数量统计
    news_count: NewsCount,
    /// 最近发布的新闻
    recent_news: RecentNews,
}

impl GameResponse {
    /// 将应用层游戏摘要转换为公开游戏响应
    pub(super) fn from_summary(value: GameSummary, asset_base_url: &str) -> Self {
        let game_cover = value.cover.clone();
        let icon_variants = game_icon_variants(value.icon.as_deref(), asset_base_url);

        Self {
            id: value.id,
            name: value.name_zh,
            index: value.index,
            cover: public_asset_url(asset_base_url, value.cover),
            icon: public_asset_url(asset_base_url, value.icon),
            icon_variants,
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

/// 根据内置游戏图标路径构造各尺寸变体
fn game_icon_variants(icon: Option<&str>, asset_base_url: &str) -> Vec<GameIconVariantResponse> {
    let Some(prefix) = icon
        .filter(|value| value.starts_with("/assets/games/"))
        .and_then(|value| value.strip_suffix("/icon.avif"))
    else {
        return Vec::new();
    };

    GAME_ICON_SIZES
        .into_iter()
        .map(|size| GameIconVariantResponse {
            size,
            url: format!("{asset_base_url}{prefix}/icon-{size}.avif"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::game_icon_variants;

    #[test]
    fn builds_public_urls_for_each_game_icon_size() {
        let variants = game_icon_variants(
            Some("/assets/games/ys/icon.avif"),
            "https://assets.example.com",
        );

        assert_eq!(
            variants
                .iter()
                .map(|variant| (variant.size, variant.url.as_str()))
                .collect::<Vec<_>>(),
            [
                (
                    64,
                    "https://assets.example.com/assets/games/ys/icon-64.avif"
                ),
                (
                    128,
                    "https://assets.example.com/assets/games/ys/icon-128.avif"
                ),
                (
                    256,
                    "https://assets.example.com/assets/games/ys/icon-256.avif"
                ),
            ]
        );
    }

    #[test]
    fn omits_variants_for_icons_without_the_bundled_asset_convention() {
        assert!(
            game_icon_variants(
                Some("https://example.com/icon.avif"),
                "https://assets.example.com"
            )
            .is_empty()
        );
    }
}
