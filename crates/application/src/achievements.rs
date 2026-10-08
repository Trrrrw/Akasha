use serde::{Deserialize, Serialize};

use crate::{
    ApplicationError, ApplicationRepository, ApplicationServices,
    game_data::{GameDataCollectionFilter, GameDataListFilter},
    search::TextQuery,
};

/// 网站使用的成就定义，不包含玩家完成状态或来源结构
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub group_id: String,
    pub group_name: String,
    pub group_order: i64,
    pub order: i64,
    pub hidden: Option<bool>,
    pub rewards: Vec<AchievementReward>,
    pub target: Option<u64>,
    pub previous_id: Option<String>,
}

/// 成就奖励中的物品及数量
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AchievementReward {
    pub item_id: String,
    pub name: Option<String>,
    pub count: u64,
}

/// 从成就目录聚合的分类
#[derive(Debug, Clone)]
pub struct AchievementGroup {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub total: u64,
}

/// 成就的业务筛选条件
#[derive(Debug, Clone)]
pub struct AchievementListFilter {
    pub group_id: Option<String>,
    pub hidden: Option<bool>,
}

impl Achievement {
    /// 校验入库定义，来源文件和未映射字段不能混入公开模型
    pub fn validate(&self) -> Result<(), ApplicationError> {
        for (label, value, max) in [
            ("id", &self.id, 128),
            ("name", &self.name, 1024),
            ("group_id", &self.group_id, 128),
            ("group_name", &self.group_name, 1024),
        ] {
            if value.trim().is_empty() || value.len() > max {
                return Err(ApplicationError::InvalidInput(format!(
                    "invalid achievement {label}"
                )));
            }
        }
        if self.description.len() > 16_384
            || self.rewards.len() > 32
            || self
                .previous_id
                .as_ref()
                .is_some_and(|id| id.len() > 128 || id == &self.id)
            || self.target == Some(0)
        {
            return Err(ApplicationError::InvalidInput(
                "invalid achievement definition".to_owned(),
            ));
        }
        if self.rewards.iter().any(|reward| {
            reward.item_id.is_empty()
                || reward.item_id.len() > 128
                || reward.count == 0
                || reward.name.as_ref().is_some_and(|name| name.len() > 1024)
        }) {
            return Err(ApplicationError::InvalidInput(
                "invalid achievement reward".to_owned(),
            ));
        }
        Ok(())
    }
}

impl<R: ApplicationRepository> ApplicationServices<R> {
    /// 分页读取规范化成就，复用游戏数据持久化机制
    pub async fn list_achievements(
        &self,
        game_id: String,
        query: Option<TextQuery>,
        filter: AchievementListFilter,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<Achievement>), ApplicationError> {
        let (total, entries) = self
            .repository
            .list_game_data(GameDataListFilter {
                game_id,
                collection: "achievement".to_owned(),
                query,
                collection_filter: Some(GameDataCollectionFilter::Achievement(filter)),
                limit,
                offset,
            })
            .await?;
        let items = entries
            .into_iter()
            .map(|entry| {
                serde_json::from_value(entry.summary).map_err(|error| {
                    ApplicationError::InvariantViolation(format!(
                        "invalid stored achievement: {error}"
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((total, items))
    }

    /// 获取成就分类及每类数量
    pub async fn list_achievement_groups(
        &self,
        game_id: &str,
    ) -> Result<Vec<AchievementGroup>, ApplicationError> {
        Ok(self.repository.list_achievement_groups(game_id).await?)
    }
}
