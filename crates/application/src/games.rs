use crate::{ApplicationError, ApplicationRepository, ApplicationServices};

use super::news::{NewsCount, RecentNews};

/// 公开游戏接口所需的游戏及新闻摘要数据
#[derive(Debug, Clone)]
pub struct GameSummary {
    pub id: String,
    pub name_en: String,
    pub name_zh: String,
    pub index: i64,
    pub cover: Option<String>,
    pub icon: Option<String>,
    pub news_count: NewsCount,
    pub recent_news: RecentNews,
}

impl<R> ApplicationServices<R>
where
    R: ApplicationRepository,
{
    /// 按配置的展示顺序列出所有游戏
    pub async fn list_games(&self) -> Result<Vec<GameSummary>, ApplicationError> {
        Ok(self.repository.list_games().await?)
    }

    /// 按公开标识查找一个游戏
    pub async fn find_game(&self, game_id: &str) -> Result<Option<GameSummary>, ApplicationError> {
        Ok(self.repository.find_game(game_id).await?)
    }
}
