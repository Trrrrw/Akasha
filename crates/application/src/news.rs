use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use crate::{ApplicationError, ApplicationRepository, ApplicationServices, audit::AuditContext};

/// 视频详情默认返回的相关视频数量
const RELATED_VIDEO_LIMIT: u64 = 8;

/// 新闻集合的文章与视频聚合数量
#[derive(Debug, Clone, Copy, Default)]
pub struct NewsCount {
    pub total: u64,
    pub article: u64,
    pub video: u64,
}

/// 新闻集合中最新的文章和视频
#[derive(Debug, Clone, Default)]
pub struct RecentNews {
    pub article: Vec<NewsSummary>,
    pub video: Vec<NewsSummary>,
}

/// 已配置的游戏新闻来源
#[derive(Debug, Clone)]
pub struct NewsSource {
    pub id: String,
    pub name: String,
    pub index: i64,
}

/// 用于筛选和分页一个游戏及来源下的新闻
#[derive(Debug, Clone)]
pub struct ListNewsFilter {
    pub source_id: String,
    pub game_id: String,
    pub query: Option<String>,
    pub tags: Option<Vec<String>>,
    pub news_type: Option<String>,
    pub start_publish_time: Option<DateTime<FixedOffset>>,
    pub end_publish_time: Option<DateTime<FixedOffset>>,
    pub limit: u64,
    pub offset: u64,
    pub reverse: bool,
}

/// 不依赖持久化实现的新闻读取模型
#[derive(Debug, Clone)]
pub struct NewsSummary {
    pub id: String,
    /// 新闻来源标识，用于选择对应的外部资源处理规则
    pub source_id: String,
    pub title: String,
    pub publish_time: DateTime<FixedOffset>,
    pub source_url: String,
    pub cover: Option<String>,
    pub news_type: String,
    pub tags: Vec<String>,
    pub video_url: Option<String>,
    /// 视频时长，单位为毫秒
    pub video_duration_ms: Option<i64>,
    pub intro: Option<String>,
}

/// 用于维护任务的新闻原始数据和当前投影
#[derive(Debug, Clone)]
pub struct NewsRawItem {
    pub id: String,
    pub title: String,
    pub intro: Option<String>,
    pub publish_time: DateTime<FixedOffset>,
    pub source_url: String,
    pub cover: Option<String>,
    pub news_type: String,
    pub tags: Vec<String>,
    pub video_url: Option<String>,
    pub video_duration_ms: Option<i64>,
    pub raw_data: Value,
}

/// 按来源读取原始新闻的筛选条件
#[derive(Debug, Clone)]
pub struct ListNewsRawFilter {
    pub game_id: String,
    pub source_id: String,
    pub news_id: Option<String>,
    pub after_id: Option<String>,
    pub news_type: Option<String>,
    pub limit: u64,
}

/// 包含新闻统计和最近新闻的标签
#[derive(Debug, Clone)]
pub struct NewsTag {
    pub name: String,
    pub index: i64,
    pub group: Option<String>,
    pub group_index: Option<i64>,
    pub news_count: NewsCount,
    pub recent: RecentNews,
}

/// 包含游戏兜底封面的新闻分页结果
#[derive(Debug, Clone)]
pub struct NewsListResult {
    pub total: u64,
    pub items: Vec<NewsSummary>,
    pub game_cover: Option<String>,
}

/// 一条新闻及其游戏的兜底封面资源
#[derive(Debug, Clone)]
pub struct NewsItemResult {
    pub item: NewsSummary,
    pub game_cover: Option<String>,
}

/// 一条公开新闻详情及视频相关推荐
#[derive(Debug, Clone)]
pub struct NewsDetailResult {
    pub item: NewsSummary,
    pub game_cover: Option<String>,
    /// 仅视频新闻包含相关推荐，普通文章为空
    pub related_videos: Vec<NewsSummary>,
}

/// 标签结果及其游戏的兜底封面资源
#[derive(Debug, Clone)]
pub struct NewsTagsResult {
    pub tags: Vec<NewsTag>,
    pub game_cover: Option<String>,
}

/// 将一个新闻标签作为媒体剧集展示所需的摘要
#[derive(Debug, Clone)]
pub struct NewsSeries {
    /// 标签名称
    pub tag_name: String,
    /// 游戏中文名称
    pub game_name: String,
    /// 剧集封面，优先使用首个视频封面
    pub cover: Option<String>,
    /// 首个视频的发布时间
    pub premiered: DateTime<FixedOffset>,
    /// 标签下的视频总数
    pub episode_count: u64,
}

/// 创建或更新一条新闻并替换其标签关联
#[derive(Debug, Clone)]
pub struct UpdateNewsCommand {
    pub game_id: String,
    pub source_id: String,
    pub id: String,
    pub title: String,
    pub intro: Option<String>,
    pub publish_time: DateTime<FixedOffset>,
    pub source_url: String,
    pub cover: Option<String>,
    pub news_type: String,
    pub video_url: Option<String>,
    /// 视频时长，单位为毫秒
    pub video_duration_ms: Option<i64>,
    pub tags: Vec<String>,
    pub raw_data: Value,
    pub audit: AuditContext,
}

/// 创建或更新新闻后的结果
#[derive(Debug, Clone)]
pub struct UpdateNewsResult {
    pub item: NewsSummary,
    pub created: bool,
}

/// 包含兜底封面的新闻创建或更新结果
#[derive(Debug, Clone)]
pub struct UpdatedNewsResult {
    pub item: NewsSummary,
    pub created: bool,
    pub game_cover: Option<String>,
}

/// 同步一个游戏及来源的标签目录
#[derive(Debug, Clone)]
pub struct SyncNewsTagsCommand {
    pub game_id: String,
    pub source_id: String,
    pub tags: Vec<NewsTagInput>,
    pub audit: AuditContext,
}

/// 同步来源标签目录时提供的单个标签
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsTagInput {
    pub name: String,
    pub index: i64,
    pub group: Option<String>,
    pub group_index: Option<i64>,
}

/// 同步来源标签目录后的执行结果
#[derive(Debug, Clone)]
pub struct SyncNewsTagsResult {
    pub changed: bool,
    pub tags: Vec<NewsTagInput>,
}

/// 替换一个来源多条新闻的标签关联
#[derive(Debug, Clone)]
pub struct ReplaceNewsTagsCommand {
    pub game_id: String,
    pub source_id: String,
    pub updates: Vec<NewsTagUpdate>,
    pub audit: AuditContext,
}

/// 一条新闻的替换标签集合
#[derive(Debug, Clone)]
pub struct NewsTagUpdate {
    pub id: String,
    pub tags: Vec<String>,
}

impl<R> ApplicationServices<R>
where
    R: ApplicationRepository,
{
    /// 列出一个游戏已配置的新闻来源
    pub async fn list_news_sources(
        &self,
        game_id: &str,
    ) -> Result<Vec<NewsSource>, ApplicationError> {
        Ok(self.repository.list_news_sources(game_id).await?)
    }

    /// 列出来源标签及游戏兜底封面
    pub async fn list_news_tags(
        &self,
        game_id: &str,
        source_id: &str,
    ) -> Result<NewsTagsResult, ApplicationError> {
        let tags = self.repository.list_news_tags(game_id, source_id).await?;
        let game_cover = self.repository.find_game_cover(game_id).await?;

        Ok(NewsTagsResult { tags, game_cover })
    }

    /// 查找可作为媒体剧集导出的标签
    pub async fn find_news_series(
        &self,
        game_id: &str,
        source_id: &str,
        tag_name: &str,
    ) -> Result<Option<NewsSeries>, ApplicationError> {
        Ok(self
            .repository
            .find_news_series(game_id, source_id, tag_name)
            .await?)
    }

    /// 列出新闻并解析请求游戏的兜底封面
    pub async fn list_news(
        &self,
        filter: ListNewsFilter,
    ) -> Result<NewsListResult, ApplicationError> {
        let game_id = filter.game_id.clone();
        let (total, items) = self.repository.list_news(filter).await?;
        let game_cover = self.repository.find_game_cover(&game_id).await?;

        Ok(NewsListResult {
            total,
            items,
            game_cover,
        })
    }

    /// 读取维护任务需要的原始新闻分页
    pub async fn list_news_raw(
        &self,
        filter: ListNewsRawFilter,
    ) -> Result<(u64, Vec<NewsRawItem>), ApplicationError> {
        Ok(self.repository.list_news_raw(filter).await?)
    }

    /// 查找新闻并在存在时解析兜底封面
    pub async fn find_news(
        &self,
        game_id: &str,
        source_id: &str,
        news_id: &str,
    ) -> Result<Option<NewsItemResult>, ApplicationError> {
        let Some(item) = self
            .repository
            .find_news(game_id, source_id, news_id)
            .await?
        else {
            return Ok(None);
        };
        let game_cover = self.repository.find_game_cover(game_id).await?;

        Ok(Some(NewsItemResult { item, game_cover }))
    }

    /// 查找公开新闻详情并为视频附加相关推荐
    pub async fn find_news_detail(
        &self,
        game_id: &str,
        source_id: &str,
        news_id: &str,
    ) -> Result<Option<NewsDetailResult>, ApplicationError> {
        let Some(NewsItemResult { item, game_cover }) =
            self.find_news(game_id, source_id, news_id).await?
        else {
            return Ok(None);
        };

        // 只有视频详情需要额外执行相关推荐查询
        let related_videos = if item.news_type == "video" {
            self.list_related_videos(game_id, source_id, news_id, &item.tags)
                .await?
        } else {
            Vec::new()
        };

        Ok(Some(NewsDetailResult {
            item,
            game_cover,
            related_videos,
        }))
    }

    /// 按共同标签列出同一游戏及来源的相关视频
    pub async fn list_related_videos(
        &self,
        game_id: &str,
        source_id: &str,
        news_id: &str,
        tags: &[String],
    ) -> Result<Vec<NewsSummary>, ApplicationError> {
        Ok(self
            .repository
            .list_related_videos(game_id, source_id, news_id, tags, RELATED_VIDEO_LIMIT)
            .await?)
    }

    /// 创建或更新新闻并解析其游戏的兜底封面
    pub async fn update_news(
        &self,
        command: UpdateNewsCommand,
    ) -> Result<UpdatedNewsResult, ApplicationError> {
        let game_id = command.game_id.clone();
        let result = self.repository.update_news(command).await?;
        let game_cover = self.repository.find_game_cover(&game_id).await?;

        Ok(UpdatedNewsResult {
            item: result.item,
            created: result.created,
            game_cover,
        })
    }

    /// 同步一个来源的标签目录
    pub async fn sync_news_tags(
        &self,
        command: SyncNewsTagsCommand,
    ) -> Result<SyncNewsTagsResult, ApplicationError> {
        Ok(self.repository.sync_news_tags(command).await?)
    }

    /// 替换一个来源多条新闻的标签关联
    pub async fn replace_news_tags(
        &self,
        command: ReplaceNewsTagsCommand,
    ) -> Result<(), ApplicationError> {
        self.repository.replace_news_tags(command).await?;
        Ok(())
    }
}
