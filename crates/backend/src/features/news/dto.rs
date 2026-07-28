use akasha_db::repositories::{
    news::{NewsSummary, projections::NewsSourceProjection},
    news_tags::NewsTagProjection,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::http::response::public_asset_url;

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻数量统计")]
pub struct NewsCount {
    pub(crate) total: u64,
    pub(crate) article: u64,
    pub(crate) video: u64,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "标签的最近新闻")]
pub struct RecentNews {
    /// 最新文章
    pub(crate) article: Vec<NewsItemResponse>,
    /// 最新视频
    pub(crate) video: Vec<NewsItemResponse>,
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
    news_count: NewsCount,
    /// 最近新闻
    recent: RecentNews,
}

impl NewsTagResponse {
    fn from_projection(
        value: NewsTagProjection,
        game_cover: Option<&str>,
        asset_base_url: &str,
    ) -> Self {
        NewsTagResponse {
            name: value.name,
            index: value.index,
            news_count: NewsCount {
                total: value.news_count.total,
                article: value.news_count.article,
                video: value.news_count.video,
            },
            recent: RecentNews {
                article: value
                    .recent
                    .article
                    .into_iter()
                    .map(|news| NewsItemResponse::from_summary(news, game_cover, asset_base_url))
                    .collect(),
                video: value
                    .recent
                    .video
                    .into_iter()
                    .map(|news| NewsItemResponse::from_summary(news, game_cover, asset_base_url))
                    .collect(),
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
        game_cover: Option<&str>,
        asset_base_url: &str,
    ) -> Self {
        let mut groups: Vec<NewsTagGroupResponse> = Vec::new();

        for row in rows {
            let group_name = row.group.clone();
            let group_index = row.group_index;
            let tag = NewsTagResponse::from_projection(row, game_cover, asset_base_url);

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
pub(crate) struct NewsItemResponse {
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
    pub(crate) fn from_summary(
        value: NewsSummary,
        game_cover: Option<&str>,
        asset_base_url: &str,
    ) -> Self {
        Self {
            id: value.id,
            title: value.title,
            publish_time: Some(value.publish_time.to_rfc3339()),
            source_url: value.source_url,
            cover: public_asset_url(
                asset_base_url,
                value.cover.or_else(|| game_cover.map(str::to_owned)),
            ),
            news_type: value.news_type,
            tags: value.tags,
            video_url: value.video_url,
            intro: value.intro,
        }
    }
}
