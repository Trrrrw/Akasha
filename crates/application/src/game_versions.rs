use chrono::{DateTime, FixedOffset};

use crate::{ApplicationError, ApplicationRepository, ApplicationServices, audit::AuditContext};

/// 游戏版本及其有效时间范围
#[derive(Debug, Clone)]
pub struct GameVersion {
    pub game_id: String,
    pub id: String,
    pub name: Option<String>,
    pub start_time: DateTime<FixedOffset>,
    pub end_time: Option<DateTime<FixedOffset>>,
    pub time_status: String,
    pub source_id: String,
    pub source_news_id: String,
    pub source_hash: String,
}

/// Worker 提交的版本投影
#[derive(Debug, Clone)]
pub struct GameVersionInput {
    pub id: String,
    pub name: Option<String>,
    pub start_time: DateTime<FixedOffset>,
    pub time_status: String,
    pub source_id: String,
    pub source_news_id: String,
    pub source_hash: String,
}

/// 同步一个游戏版本时间线的命令
#[derive(Debug, Clone)]
pub struct SyncGameVersionsCommand {
    pub game_id: String,
    pub replace: bool,
    pub versions: Vec<GameVersionInput>,
    pub audit: AuditContext,
}

/// 游戏版本时间线同步后的变化统计
#[derive(Debug, Clone, Copy)]
pub struct SyncGameVersionsResult {
    pub versions_created: u64,
    pub versions_updated: u64,
    pub versions_deleted: u64,
    pub changed: bool,
}

impl<R> ApplicationServices<R>
where
    R: ApplicationRepository,
{
    /// 按开始时间列出一个游戏的版本时间线
    pub async fn list_game_versions(
        &self,
        game_id: &str,
    ) -> Result<Vec<GameVersion>, ApplicationError> {
        Ok(self.repository.list_game_versions(game_id).await?)
    }

    /// 校验并同步一个游戏的版本时间线
    pub async fn sync_game_versions(
        &self,
        command: SyncGameVersionsCommand,
    ) -> Result<SyncGameVersionsResult, ApplicationError> {
        validate_sync_command(&command)?;
        Ok(self.repository.sync_game_versions(command).await?)
    }
}

fn validate_sync_command(command: &SyncGameVersionsCommand) -> Result<(), ApplicationError> {
    if command.game_id.trim().is_empty() {
        return Err(ApplicationError::InvalidInput(
            "game_id must not be empty".to_owned(),
        ));
    }
    for version in &command.versions {
        if version.id.trim().is_empty()
            || version.source_news_id.trim().is_empty()
            || version.source_hash.trim().is_empty()
            || !matches!(version.time_status.as_str(), "scheduled" | "confirmed")
        {
            return Err(ApplicationError::InvalidInput(
                "invalid game version projection".to_owned(),
            ));
        }
    }
    Ok(())
}
