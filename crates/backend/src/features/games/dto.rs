use akasha_db::repositories::games::GameSummary;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻数量统计")]
struct NewsStatsResponse {
    total: u64,
    video: u64,
    article: u64,
    latest_publish_time: Option<String>,
    latest_video_publish_time: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "游戏基础信息")]
pub(super) struct GameResponse {
    id: String,
    name: String,
    index: i64,
    cover: Option<String>,
    icon: Option<String>,
    news_stats: NewsStatsResponse,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "游戏详细信息")]
pub(super) struct GameDetailResponse {
    summary: GameResponse,
}

impl From<GameSummary> for GameResponse {
    fn from(value: GameSummary) -> Self {
        Self {
            id: value.id,
            name: value.name_zh,
            index: value.index,
            cover: value.cover,
            icon: value.icon,
            news_stats: NewsStatsResponse {
                total: value.news_stats.total,
                video: value.news_stats.video,
                article: value.news_stats.article,
                latest_publish_time: value
                    .news_stats
                    .latest_publish_time
                    .map(|time| time.to_rfc3339()),
                latest_video_publish_time: value
                    .news_stats
                    .latest_video_publish_time
                    .map(|time| time.to_rfc3339()),
            },
        }
    }
}

impl From<GameSummary> for GameDetailResponse {
    fn from(value: GameSummary) -> Self {
        Self {
            summary: GameResponse::from(value),
        }
    }
}
