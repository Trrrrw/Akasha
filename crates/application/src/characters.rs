use serde_json::Value;

use crate::audit::AuditContext;

use crate::{ApplicationError, ApplicationRepository, ApplicationServices};

/// 用于筛选和分页一个游戏的角色
#[derive(Debug, Clone)]
pub struct CharacterListFilter {
    pub game_id: String,
    pub query: Option<String>,
    pub gender: Option<String>,
    pub voice_actor: Option<String>,
    pub birthday_month: Option<i16>,
    pub limit: u64,
    pub offset: u64,
}

/// 不依赖持久化实现的角色读取模型
#[derive(Debug, Clone)]
pub struct CharacterSummary {
    pub id: String,
    pub item_id: String,
    pub name: String,
    pub description: Option<String>,
    pub gender: Option<String>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub voice_actor: Option<String>,
}

/// 替换一个游戏的完整角色目录
#[derive(Debug, Clone)]
pub struct SyncCharactersCommand {
    pub game_id: String,
    pub items: Vec<SyncCharacterItem>,
    pub audit: AuditContext,
}

/// 目录同步时提供的单个角色
#[derive(Debug, Clone)]
pub struct SyncCharacterItem {
    pub id: String,
    pub item_id: String,
    pub name: String,
    pub description: Option<String>,
    pub gender: Option<String>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub voice_actor: Option<String>,
    pub extra: Value,
}

/// 角色目录同步的执行结果
#[derive(Debug, Clone, Copy)]
pub struct SyncCharactersResult {
    pub created: u64,
    pub updated: u64,
    pub deleted: u64,
    pub changed: bool,
    pub total: u64,
}

impl<R> ApplicationServices<R>
where
    R: ApplicationRepository,
{
    /// 列出符合给定筛选条件的角色
    pub async fn list_characters(
        &self,
        filter: CharacterListFilter,
    ) -> Result<(u64, Vec<CharacterSummary>), ApplicationError> {
        Ok(self.repository.list_characters(filter).await?)
    }

    /// 同步一个游戏提供的全部角色
    pub async fn sync_characters(
        &self,
        command: SyncCharactersCommand,
    ) -> Result<SyncCharactersResult, ApplicationError> {
        Ok(self.repository.sync_characters(command).await?)
    }
}
