use akasha_db::repositories::{
    news::{NewsSummary, projections::NewsSourceProjection},
    news_tags::NewsTagProjection,
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻数量统计")]
pub struct NewsStats {
    total: u64,
    video: u64,
    article: u64,
    latest_publish_time: Option<String>,
    latest_video_publish_time: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻来源基础信息")]
pub(super) struct NewsSourceResponse {
    /// 来源 ID
    id: String,
    /// 来源名称
    name: String,
    /// 排序
    index: i64,
}

impl From<NewsSourceProjection> for NewsSourceResponse {
    fn from(value: NewsSourceProjection) -> Self {
        NewsSourceResponse {
            id: value.id,
            name: value.name,
            index: value.index,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻标签信息")]
pub(super) struct NewsTagResponse {
    /// 标签名
    name: String,
    /// 排序
    index: i64,
    /// 新闻数量
    news_count: NewsStats,
}

impl From<NewsTagProjection> for NewsTagResponse {
    fn from(value: NewsTagProjection) -> Self {
        NewsTagResponse {
            name: value.name,
            index: value.index,
            news_count: NewsStats {
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

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻标签组")]
pub(super) struct NewsTagGroupResponse {
    /// 标签组名，未分组标签为 null
    name: Option<String>,
    /// 分类组排序，未分组分类为 null
    index: Option<i64>,
    /// 组内标签
    tags: Vec<NewsTagResponse>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻来源的标签列表")]
pub(super) struct NewsTagsResponse {
    /// 游戏 ID
    game_id: String,
    /// 来源 ID
    source_id: String,
    /// 标签组
    groups: Vec<NewsTagGroupResponse>,
}

impl NewsTagsResponse {
    /// 将已排序的标签投影聚合为标签组
    pub(super) fn from_rows(
        game_id: String,
        source_id: String,
        rows: Vec<NewsTagProjection>,
    ) -> Self {
        let mut groups: Vec<NewsTagGroupResponse> = Vec::new();

        for row in rows {
            let group_name = row.group.clone();
            let group_index = row.group_index;
            let tag = NewsTagResponse::from(row);

            if let Some(group) = groups.last_mut()
                && group.name == group_name
                && group.index == group_index
            {
                group.tags.push(tag);
                continue;
            }

            groups.push(NewsTagGroupResponse {
                name: group_name,
                index: group_index,
                tags: vec![tag],
            });
        }

        Self {
            game_id,
            source_id,
            groups,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻列表上下文")]
pub(super) struct NewsListMeta {
    /// 来源 ID
    source_id: String,
    /// 游戏 ID
    game_id: String,
}

impl NewsListMeta {
    pub(super) fn new(source_id: String, game_id: String) -> Self {
        Self { source_id, game_id }
    }
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻基础信息")]
pub(super) struct NewsItemResponse {
    id: String,
    title: String,
    publish_time: Option<String>,
    source_url: String,
    cover: Option<String>,
    news_type: String,
    tags: Vec<String>,
    video_url: Option<String>,
    intro: Option<String>,
}

impl NewsItemResponse {
    pub(super) fn from_summary(value: NewsSummary, game_cover: Option<&str>) -> Self {
        Self {
            id: value.id,
            title: value.title,
            publish_time: Some(value.publish_time.to_rfc3339()),
            source_url: value.source_url,
            cover: value.cover.or_else(|| game_cover.map(str::to_owned)),
            news_type: value.news_type,
            tags: value.tags,
            video_url: value.video_url,
            intro: value.intro,
        }
    }
}
