use akasha_application::news::{NewsDetailResult, NewsSource, NewsSummary, NewsTag};
use serde::Serialize;
use utoipa::ToSchema;

use super::{china_timezone, video_duration_seconds};
use crate::http::response::public_asset_url;

/// 公开新闻数量统计
#[derive(Serialize, ToSchema)]
#[schema(description = "新闻数量统计")]
pub struct NewsCount {
    /// 新闻总数
    pub(crate) total: u64,
    /// 文章数量
    pub(crate) article: u64,
    /// 视频数量
    pub(crate) video: u64,
}

/// 公开新闻最近条目集合
#[derive(Serialize, ToSchema)]
#[schema(description = "标签的最近新闻")]
pub struct RecentNews {
    /// 最新文章
    pub(crate) article: Vec<NewsItemResponse>,
    /// 最新视频
    pub(crate) video: Vec<NewsItemResponse>,
}

/// 公开新闻来源响应
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

impl From<NewsSource> for NewsSourceResponse {
    /// 将应用层新闻来源转换为公开响应
    fn from(value: NewsSource) -> Self {
        NewsSourceResponse {
            id: value.id,
            name: value.name,
            index: value.index,
        }
    }
}

/// 公开新闻标签响应
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
    /// 将应用层标签及新闻摘要转换为公开标签响应
    fn from_projection(value: NewsTag, game_cover: Option<&str>, asset_base_url: &str) -> Self {
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

/// 公开新闻标签分组响应
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

/// 一个游戏新闻来源的公开标签列表响应
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
    /// 将已排序的应用层标签聚合为公开标签组
    pub(super) fn from_rows(
        game_id: String,
        source_id: String,
        rows: Vec<NewsTag>,
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

/// 新闻列表的来源和游戏上下文
#[derive(Serialize, ToSchema)]
#[schema(description = "新闻列表上下文")]
pub(super) struct NewsListMeta {
    /// 来源 ID
    source_id: String,
    /// 游戏 ID
    game_id: String,
}

impl NewsListMeta {
    /// 创建新闻列表响应的来源和游戏上下文
    pub(super) fn new(source_id: String, game_id: String) -> Self {
        Self { source_id, game_id }
    }
}

/// 公开新闻条目响应
#[derive(Serialize, ToSchema)]
#[schema(description = "新闻基础信息")]
pub(crate) struct NewsItemResponse {
    /// 新闻 ID
    id: String,
    /// 新闻来源 ID，米游社视频解析接口需要使用该来源上下文
    source_id: String,
    /// 新闻标题
    title: String,
    /// RFC 3339 格式的发布时间
    publish_time: Option<String>,
    /// 新闻原始页面地址
    source_url: String,
    /// 新闻封面绝对地址
    cover: Option<String>,
    /// 新闻类型，article 或 video
    news_type: String,
    /// 新闻标签
    tags: Vec<String>,
    /// 非米游社来源保存的视频地址，米游社视频需要通过专用接口获取
    video_url: Option<String>,
    /// 视频时长，单位为秒
    video_duration: Option<u64>,
    /// 新闻简介，可能包含来源 HTML
    intro: Option<String>,
}

/// 相关视频摘要响应
#[derive(Serialize, ToSchema)]
#[schema(description = "相关视频摘要")]
pub(crate) struct RelatedVideoResponse {
    /// 新闻 ID
    id: String,
    /// 新闻来源 ID
    source_id: String,
    /// 视频标题
    title: String,
    /// 发布时间
    publish_time: String,
    /// 视频封面
    cover: Option<String>,
    /// 视频时长，单位为秒
    video_duration: Option<u64>,
    /// 新闻标签
    tags: Vec<String>,
}

impl RelatedVideoResponse {
    /// 将应用层新闻摘要转换为相关视频响应
    fn from_summary(value: NewsSummary, game_cover: Option<&str>, asset_base_url: &str) -> Self {
        let china_timezone = china_timezone();

        Self {
            id: value.id,
            source_id: value.source_id,
            title: value.title,
            publish_time: value
                .publish_time
                .with_timezone(&china_timezone)
                .to_rfc3339(),
            cover: public_asset_url(
                asset_base_url,
                value.cover.or_else(|| game_cover.map(str::to_owned)),
            ),
            video_duration: value.video_duration_ms.and_then(video_duration_seconds),
            tags: value.tags,
        }
    }
}

/// 公开新闻详情响应
#[derive(Serialize, ToSchema)]
#[schema(description = "新闻完整信息及视频相关推荐")]
pub(crate) struct NewsDetailResponse {
    /// 新闻基础信息
    #[serde(flatten)]
    item: NewsItemResponse,
    /// 按共同标签和发布时间排序的相关视频，普通文章为空
    related_videos: Vec<RelatedVideoResponse>,
}

impl NewsDetailResponse {
    /// 将应用层新闻详情转换为公开响应
    pub(crate) fn from_result(value: NewsDetailResult, asset_base_url: &str) -> Self {
        let game_cover = value.game_cover.as_deref();

        Self {
            item: NewsItemResponse::from_summary(value.item, game_cover, asset_base_url),
            related_videos: value
                .related_videos
                .into_iter()
                .map(|video| RelatedVideoResponse::from_summary(video, game_cover, asset_base_url))
                .collect(),
        }
    }
}

/// 新闻视频播放地址响应
#[derive(Serialize, ToSchema)]
#[schema(description = "新闻当前可用的视频播放地址")]
pub(crate) struct NewsVideoResponse {
    /// 视频播放地址，米游社来源包含临时签名
    video_url: String,
}

impl NewsVideoResponse {
    /// 创建新闻视频播放地址响应
    pub(crate) fn new(video_url: String) -> Self {
        Self { video_url }
    }
}

impl NewsItemResponse {
    /// 将应用层新闻摘要转换为公开新闻响应
    pub(crate) fn from_summary(
        value: NewsSummary,
        game_cover: Option<&str>,
        asset_base_url: &str,
    ) -> Self {
        let china_timezone = china_timezone();

        // 米游社保存的是未签名地址，公开响应只暴露专用播放接口
        let video_url = if value.source_id == "mys" {
            None
        } else {
            value.video_url
        };

        Self {
            id: value.id,
            source_id: value.source_id,
            title: value.title,
            publish_time: Some(
                value
                    .publish_time
                    .with_timezone(&china_timezone)
                    .to_rfc3339(),
            ),
            source_url: value.source_url,
            cover: public_asset_url(
                asset_base_url,
                value.cover.or_else(|| game_cover.map(str::to_owned)),
            ),
            news_type: value.news_type,
            tags: value.tags,
            video_url,
            video_duration: value.video_duration_ms.and_then(video_duration_seconds),
            intro: value.intro,
        }
    }
}

#[cfg(test)]
mod tests {
    use akasha_application::news::{NewsDetailResult, NewsSummary};
    use chrono::Utc;

    use super::{NewsDetailResponse, NewsItemResponse, NewsVideoResponse};

    /// 构造用于响应转换测试的新闻摘要
    fn summary(source_id: &str, video_url: Option<&str>) -> NewsSummary {
        NewsSummary {
            id: "news-1".to_owned(),
            source_id: source_id.to_owned(),
            title: "测试新闻".to_owned(),
            publish_time: Utc::now().fixed_offset(),
            source_url: "https://example.com/news-1".to_owned(),
            cover: None,
            news_type: "video".to_owned(),
            tags: Vec::new(),
            video_url: video_url.map(ToOwned::to_owned),
            video_duration_ms: None,
            intro: None,
        }
    }

    /// 列表响应不泄露需要临时签名的米游社原始地址
    #[test]
    fn hides_raw_mys_video_url() {
        let response = NewsItemResponse::from_summary(
            summary("mys", Some("https://video.example/news-1.mp4")),
            None,
            "https://assets.example.com",
        );

        assert_eq!(response.video_url, None);
    }

    /// 非米游社视频可以直接返回已保存地址
    #[test]
    fn keeps_non_mys_video_url() {
        let response = NewsItemResponse::from_summary(
            summary("web_cn", Some("https://video.example/news-1.mp4")),
            None,
            "https://assets.example.com",
        );

        assert_eq!(
            response.video_url.as_deref(),
            Some("https://video.example/news-1.mp4")
        );
    }

    /// 毫秒时长以最接近的整秒返回
    #[test]
    fn rounds_video_duration_to_seconds() {
        let mut value = summary("web_cn", Some("https://video.example/news-1.mp4"));
        value.video_duration_ms = Some(154_633);

        let response = NewsItemResponse::from_summary(value, None, "https://assets.example.com");

        assert_eq!(response.video_duration, Some(155));
    }

    /// 新闻详情在顶层附带精简的相关视频摘要
    #[test]
    fn detail_includes_lean_related_video_summaries() {
        let item = summary("mys", Some("https://video.example/current.mp4"));
        let mut related = summary("mys", Some("https://video.example/news-1.mp4"));
        related.video_duration_ms = Some(154_633);
        related.tags = vec!["角色".to_owned(), "PV".to_owned()];

        let response = NewsDetailResponse::from_result(
            NewsDetailResult {
                item,
                game_cover: Some("/assets/game-cover.png".to_owned()),
                related_videos: vec![related],
            },
            "https://assets.example.com",
        );

        assert_eq!(response.related_videos.len(), 1);
        assert_eq!(response.related_videos[0].id, "news-1");
        assert_eq!(response.related_videos[0].source_id, "mys");
        assert_eq!(response.related_videos[0].video_duration, Some(155));
        assert_eq!(response.related_videos[0].tags, ["角色", "PV"]);
        assert_eq!(
            response.related_videos[0].cover.as_deref(),
            Some("https://assets.example.com/assets/game-cover.png")
        );
    }

    /// 播放接口只返回当前视频地址
    #[test]
    fn video_response_only_contains_video_url() {
        let response = NewsVideoResponse::new("https://video.example/current.mp4".to_owned());
        let value = serde_json::to_value(response).expect("视频响应应可序列化");

        assert_eq!(value.as_object().map(|object| object.len()), Some(1));
        assert_eq!(
            value.get("video_url").and_then(serde_json::Value::as_str),
            Some("https://video.example/current.mp4")
        );
    }
}
