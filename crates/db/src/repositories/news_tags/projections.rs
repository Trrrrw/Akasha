use crate::models::NewsStats;

pub struct NewsTagProjection {
    pub name: String,
    pub index: i64,
    pub group: Option<String>,
    pub group_index: Option<i64>,
    pub news_stats: NewsStats,
}
