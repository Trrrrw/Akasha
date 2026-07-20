use sea_orm::prelude::DateTimeWithTimeZone;

use crate::entities::news_sources;

#[derive(Debug, Clone)]
pub struct NewsSourceProjection {
    pub id: String,
    pub name: String,
    pub index: i64,
}

impl From<news_sources::Model> for NewsSourceProjection {
    fn from(value: news_sources::Model) -> Self {
        Self {
            id: value.id,
            name: value.name,
            index: value.index,
        }
    }
}

pub type NewsSourceSummary = NewsSourceProjection;

#[derive(Debug, Clone)]
pub struct NewsSourceStats {
    pub news_count: u64,
    pub latest_release_time: Option<DateTimeWithTimeZone>,
}

#[derive(Debug, Clone)]
pub struct ListNewsFilter {
    pub source_id: String,
    pub game_id: String,
    pub q: Option<String>,
    pub tags: Option<Vec<String>>,
    pub news_type: Option<String>,
    pub start_publish_time: Option<DateTimeWithTimeZone>,
    pub end_publish_time: Option<DateTimeWithTimeZone>,
    pub limit: u64,
    pub offset: u64,
    pub reverse: bool,
}

#[derive(Debug, Clone)]
pub struct NewsSummary {
    pub id: String,
    pub title: String,
    pub publish_time: DateTimeWithTimeZone,
    pub source_url: String,
    pub cover: Option<String>,
    pub news_type: String,
    /// 标签
    pub tags: Vec<String>,
    /// 视频链接
    pub video_url: Option<String>,
    /// 简介/正文
    pub intro: Option<String>,
}

pub struct UpdateNewsResult {
    pub news: NewsSummary,
    pub created: bool,
}
