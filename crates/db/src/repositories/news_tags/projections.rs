use crate::models::{NewsCount, RecentNews};

pub struct NewsTagProjection {
    pub name: String,
    pub index: i64,
    pub group: Option<String>,
    pub group_index: Option<i64>,
    pub news_count: NewsCount,
    pub recent: RecentNews,
}
