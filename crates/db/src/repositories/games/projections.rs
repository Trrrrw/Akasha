use crate::models::NewsStats;

#[derive(Debug, Clone)]
pub struct GameSummary {
    pub id: String,
    pub name_en: String,
    pub name_zh: String,
    pub index: i64,
    pub cover: Option<String>,
    pub icon: Option<String>,
    pub news_stats: NewsStats,
}
