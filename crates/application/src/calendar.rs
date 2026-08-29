use chrono::{DateTime, FixedOffset};

use crate::{ApplicationError, ApplicationRepository, ApplicationServices, audit::AuditContext};

/// 游戏日历中的一条活动
#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub game_id: String,
    pub id: String,
    pub kind: String,
    pub title: String,
    pub start_time: DateTime<FixedOffset>,
    pub end_time: DateTime<FixedOffset>,
    pub version_id: Option<String>,
    pub start_version_id: Option<String>,
    pub cover: Option<String>,
    pub labels: Vec<String>,
    pub source_id: String,
    pub source_news_id: String,
    pub source_url: String,
    pub source_hash: String,
}

/// 公共活动日历的数据库筛选条件
#[derive(Debug, Clone)]
pub struct ListCalendarEventsFilter {
    pub game_id: String,
    pub start_time: DateTime<FixedOffset>,
    pub end_time: DateTime<FixedOffset>,
    pub kinds: Vec<String>,
    pub limit: u64,
}

/// Worker 提交的活动投影
#[derive(Debug, Clone)]
pub struct CalendarEventInput {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub start_time: DateTime<FixedOffset>,
    pub end_time: DateTime<FixedOffset>,
    pub version_id: Option<String>,
    pub start_version_id: Option<String>,
    pub cover: Option<String>,
    pub labels: Vec<String>,
    pub source_id: String,
    pub source_news_id: String,
    pub source_url: String,
    pub source_hash: String,
}

/// 同步一个游戏活动投影的命令
#[derive(Debug, Clone)]
pub struct SyncCalendarEventsCommand {
    pub game_id: String,
    pub replace: bool,
    pub events: Vec<CalendarEventInput>,
    pub audit: AuditContext,
}

/// 日历投影同步后的变化统计
#[derive(Debug, Clone, Copy)]
pub struct SyncCalendarEventsResult {
    pub events_created: u64,
    pub events_updated: u64,
    pub events_deleted: u64,
    pub changed: bool,
}

impl<R> ApplicationServices<R>
where
    R: ApplicationRepository,
{
    /// 查询指定时间范围内的公开游戏活动
    pub async fn list_calendar_events(
        &self,
        filter: ListCalendarEventsFilter,
    ) -> Result<Vec<CalendarEvent>, ApplicationError> {
        if filter.start_time >= filter.end_time {
            return Err(ApplicationError::InvalidInput(
                "calendar start time must be before end time".to_owned(),
            ));
        }
        Ok(self.repository.list_calendar_events(filter).await?)
    }

    /// 校验并同步一个游戏的活动投影
    pub async fn sync_calendar_events(
        &self,
        command: SyncCalendarEventsCommand,
    ) -> Result<SyncCalendarEventsResult, ApplicationError> {
        validate_sync_command(&command)?;
        Ok(self.repository.sync_calendar_events(command).await?)
    }
}

fn validate_sync_command(command: &SyncCalendarEventsCommand) -> Result<(), ApplicationError> {
    if command.game_id.trim().is_empty() {
        return Err(ApplicationError::InvalidInput(
            "game_id must not be empty".to_owned(),
        ));
    }
    for event in &command.events {
        if event.id.trim().is_empty()
            || event.title.trim().is_empty()
            || event.source_news_id.trim().is_empty()
            || event.source_hash.trim().is_empty()
            || event.start_time >= event.end_time
            || !matches!(
                event.kind.as_str(),
                "game_activity" | "banner" | "web_activity"
            )
        {
            return Err(ApplicationError::InvalidInput(
                "invalid calendar event projection".to_owned(),
            ));
        }
    }
    Ok(())
}
