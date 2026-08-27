use akasha_application::calendar::{CalendarEvent, ListCalendarEventsFilter};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Query as MultiQuery;
use chrono::{Days, FixedOffset, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{
    china_timezone,
    ics::{AlarmOffset, AlarmRelation, IcsCalendar, IcsEvent},
};
use crate::{
    http::{
        error::AppError,
        path::{GamePath, require_game},
        response::{ErrorResponse, ListResponse, public_asset_url, utc_timestamp},
    },
    state::AppState,
};

const EVENT_LIMIT: u64 = 2_000;
const DEFAULT_PAST_DAYS: u64 = 30;
const DEFAULT_FUTURE_DAYS: u64 = 366;
const MAX_RANGE_DAYS: i64 = 1_100;
const MAX_REMINDER_MINUTES: u32 = 30 * 24 * 60;

/// 游戏活动日历查询参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct EventQuery {
    /// 查询开始日期，格式为 YYYY-MM-DD，默认包含最近 30 天
    from: Option<String>,
    /// 查询结束日期，格式为 YYYY-MM-DD，默认查询未来 366 天
    to: Option<String>,
    /// 活动类型，可重复传入 game_activity、banner 或 web_activity
    #[serde(default)]
    kind: Vec<String>,
}

/// 游戏活动 ICS 查询参数
#[derive(Debug, Deserialize)]
pub(super) struct EventIcsQuery {
    from: Option<String>,
    to: Option<String>,
    #[serde(default)]
    kind: Vec<String>,
    #[serde(default)]
    event_mode: EventIcsMode,
    start_reminder_minutes: Option<u32>,
    end_reminder_minutes: Option<u32>,
}

/// 游戏活动 ICS 展示与提醒选项
#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct EventIcsOptions {
    /// 事件表示方式，span 保留完整活动时段，milestones 分别生成开始和结束事件
    #[serde(default)]
    #[param(inline)]
    event_mode: EventIcsMode,
    /// 活动开始前多少分钟提醒，最大 43200 分钟
    start_reminder_minutes: Option<u32>,
    /// 活动结束前多少分钟提醒，最大 43200 分钟
    end_reminder_minutes: Option<u32>,
}

/// 游戏活动在 ICS 中的表示方式
#[derive(Debug, Default, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum EventIcsMode {
    /// 使用一条同时包含开始和结束时间的事件
    #[default]
    Span,
    /// 分别生成活动开始和活动结束事件
    Milestones,
}

/// 游戏活动日历条目
#[derive(Debug, Serialize, ToSchema)]
pub(super) struct EventResponse {
    id: String,
    kind: String,
    title: String,
    /// UTC RFC 3339 格式的活动开始时间
    start_time: String,
    /// UTC RFC 3339 格式的活动结束时间
    end_time: String,
    version: Option<String>,
    cover: Option<String>,
    labels: Vec<String>,
    url: String,
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/calendar/events",
    tag = "Calendar",
    summary = "获取游戏活动 JSON",
    description = "以 JSON 格式返回指定日期范围内的游戏活动、卡池和网页活动",
    params(GamePath, EventQuery),
    responses(
        (status = 200, body = ListResponse<EventResponse>),
        (status = 400, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn events_json(
    Path(GamePath { game_id }): Path<GamePath>,
    State(state): State<AppState>,
    MultiQuery(query): MultiQuery<EventQuery>,
) -> Result<Json<ListResponse<EventResponse>>, AppError> {
    let items = list_events(&state, &game_id, query).await?;
    Ok(Json(ListResponse {
        total: items.len() as u64,
        items,
    }))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/calendar/events.ics",
    tag = "Calendar",
    summary = "获取游戏活动 ICS",
    description = "以 ICS 格式导出指定日期范围内的游戏活动、卡池和网页活动，可选择完整时段或起止节点表示并设置提醒",
    params(GamePath, EventQuery, EventIcsOptions),
    responses(
        (status = 200, content_type = "text/calendar", body = String),
        (status = 400, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn events_ics(
    Path(GamePath { game_id }): Path<GamePath>,
    State(state): State<AppState>,
    MultiQuery(query): MultiQuery<EventIcsQuery>,
) -> Result<Response, AppError> {
    let (filter, options) = query.into_parts();
    options.validate()?;
    let items = list_events(&state, &game_id, filter).await?;
    let game = state
        .application()
        .find_game(&game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("game {game_id} not found")))?;
    let filename = format!("{game_id}-events.ics");
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
        .map_err(|error| AppError::Internal(error.into()))?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/calendar; charset=utf-8"),
            ),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        build_ics(&game_id, &game.name_zh, &items, &options),
    )
        .into_response())
}

async fn list_events(
    state: &AppState,
    game_id: &str,
    query: EventQuery,
) -> Result<Vec<EventResponse>, AppError> {
    let (start_time, end_time) = query.time_range()?;
    let kinds = query.kinds()?;
    require_game(state, game_id).await?;
    let rows = state
        .application()
        .list_calendar_events(ListCalendarEventsFilter {
            game_id: game_id.to_owned(),
            start_time,
            end_time,
            kinds,
            limit: EVENT_LIMIT,
        })
        .await?;
    Ok(rows
        .into_iter()
        .map(|event| EventResponse::from_event(event, &state.config().asset_base_url))
        .collect())
}

impl EventQuery {
    fn time_range(
        &self,
    ) -> Result<(chrono::DateTime<FixedOffset>, chrono::DateTime<FixedOffset>), AppError> {
        let timezone = china_timezone();
        let today = Utc::now().with_timezone(&timezone).date_naive();
        let from = self
            .from
            .as_deref()
            .map(parse_date)
            .transpose()?
            .unwrap_or_else(|| {
                today
                    .checked_sub_days(Days::new(DEFAULT_PAST_DAYS))
                    .unwrap_or(today)
            });
        let to = self
            .to
            .as_deref()
            .map(parse_date)
            .transpose()?
            .unwrap_or_else(|| {
                today
                    .checked_add_days(Days::new(DEFAULT_FUTURE_DAYS))
                    .unwrap_or(today)
            });
        if from >= to || (to - from).num_days() > MAX_RANGE_DAYS {
            return Err(AppError::BadRequest(
                "calendar date range must be positive and at most 1100 days".to_owned(),
            ));
        }
        let start = timezone
            .from_local_datetime(&from.and_hms_opt(0, 0, 0).expect("midnight should be valid"))
            .single()
            .expect("fixed offset should resolve local time");
        let end = timezone
            .from_local_datetime(&to.and_hms_opt(0, 0, 0).expect("midnight should be valid"))
            .single()
            .expect("fixed offset should resolve local time");
        Ok((start, end))
    }

    fn kinds(&self) -> Result<Vec<String>, AppError> {
        let mut kinds = Vec::new();
        for kind in &self.kind {
            let kind = kind.trim();
            if kind.is_empty() || kinds.iter().any(|value| value == kind) {
                continue;
            }
            if !matches!(kind, "game_activity" | "banner" | "web_activity") {
                return Err(AppError::BadRequest(
                    "kind must be game_activity, banner or web_activity".to_owned(),
                ));
            }
            kinds.push(kind.to_owned());
        }
        Ok(kinds)
    }
}

impl EventIcsOptions {
    fn validate(&self) -> Result<(), AppError> {
        if self
            .start_reminder_minutes
            .is_some_and(|minutes| minutes > MAX_REMINDER_MINUTES)
            || self
                .end_reminder_minutes
                .is_some_and(|minutes| minutes > MAX_REMINDER_MINUTES)
        {
            return Err(AppError::BadRequest(
                "calendar reminders must be between 0 and 43200 minutes".to_owned(),
            ));
        }
        Ok(())
    }
}

impl EventIcsQuery {
    fn into_parts(self) -> (EventQuery, EventIcsOptions) {
        (
            EventQuery {
                from: self.from,
                to: self.to,
                kind: self.kind,
            },
            EventIcsOptions {
                event_mode: self.event_mode,
                start_reminder_minutes: self.start_reminder_minutes,
                end_reminder_minutes: self.end_reminder_minutes,
            },
        )
    }
}

fn parse_date(value: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("calendar dates must use YYYY-MM-DD".to_owned()))
}

impl EventResponse {
    fn from_event(event: CalendarEvent, asset_base_url: &str) -> Self {
        Self {
            id: event.id,
            kind: event.kind,
            title: event.title,
            start_time: utc_timestamp(event.start_time),
            end_time: utc_timestamp(event.end_time),
            version: event.version_id,
            cover: public_asset_url(asset_base_url, event.cover),
            labels: event.labels,
            url: event.source_url,
        }
    }
}

fn build_ics(
    game_id: &str,
    game_name: &str,
    items: &[EventResponse],
    options: &EventIcsOptions,
) -> String {
    let calendar_name = format!("{game_name}游戏活动");
    let mut calendar = IcsCalendar::new("-//Akasha//Game Events//ZH-CN", &calendar_name);
    for item in items {
        let start = chrono::DateTime::parse_from_rfc3339(&item.start_time)
            .expect("stored event start should be RFC 3339")
            .with_timezone(&Utc);
        let end = chrono::DateTime::parse_from_rfc3339(&item.end_time)
            .expect("stored event end should be RFC 3339")
            .with_timezone(&Utc);
        match options.event_mode {
            EventIcsMode::Span => {
                let mut event = IcsEvent::new(&format!("event-{game_id}-{}@akasha", item.id))
                    .starts_at(start)
                    .ends_at(end)
                    .summary(&item.title)
                    .url(&item.url)
                    .transparent();
                if let Some(minutes) = options.start_reminder_minutes {
                    event = event.display_alarm(
                        AlarmRelation::Start,
                        AlarmOffset::Before(minutes),
                        &format!("活动即将开始：{}", item.title),
                    );
                }
                if let Some(minutes) = options.end_reminder_minutes {
                    event = event.display_alarm(
                        AlarmRelation::End,
                        AlarmOffset::Before(minutes),
                        &format!("活动即将结束：{}", item.title),
                    );
                }
                calendar.push_event(event);
            }
            EventIcsMode::Milestones => {
                let mut start_event =
                    IcsEvent::new(&format!("event-{game_id}-{}-start@akasha", item.id))
                        .starts_at(start)
                        .summary(&format!("【开始】{}", item.title))
                        .url(&item.url)
                        .transparent();
                if let Some(minutes) = options.start_reminder_minutes {
                    start_event = start_event.display_alarm(
                        AlarmRelation::Start,
                        AlarmOffset::Before(minutes),
                        &format!("活动即将开始：{}", item.title),
                    );
                }
                calendar.push_event(start_event);

                let mut end_event =
                    IcsEvent::new(&format!("event-{game_id}-{}-end@akasha", item.id))
                        .starts_at(end)
                        .summary(&format!("【结束】{}", item.title))
                        .url(&item.url)
                        .transparent();
                if let Some(minutes) = options.end_reminder_minutes {
                    end_event = end_event.display_alarm(
                        AlarmRelation::Start,
                        AlarmOffset::Before(minutes),
                        &format!("活动即将结束：{}", item.title),
                    );
                }
                calendar.push_event(end_event);
            }
        }
    }
    calendar.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shared_filters_and_ics_options_from_one_query() {
        let uri = "/calendar/events.ics?from=2026-08-01&to=2026-10-01&kind=game_activity&kind=banner&event_mode=milestones&start_reminder_minutes=60&end_reminder_minutes=360"
            .parse()
            .expect("test URI should be valid");
        let MultiQuery(query) = MultiQuery::<EventIcsQuery>::try_from_uri(&uri)
            .expect("combined ICS query should deserialize");
        let (filter, options) = query.into_parts();

        assert_eq!(filter.from.as_deref(), Some("2026-08-01"));
        assert_eq!(filter.to.as_deref(), Some("2026-10-01"));
        assert_eq!(filter.kind, ["game_activity", "banner"]);
        assert!(matches!(options.event_mode, EventIcsMode::Milestones));
        assert_eq!(options.start_reminder_minutes, Some(60));
        assert_eq!(options.end_reminder_minutes, Some(360));
    }

    #[test]
    fn serializes_event_json_times_in_utc() {
        let response = EventResponse::from_event(
            CalendarEvent {
                game_id: "ys".to_owned(),
                id: "event-1".to_owned(),
                kind: "game_activity".to_owned(),
                title: "映夏！归乡？千灵节！".to_owned(),
                start_time: chrono::DateTime::parse_from_rfc3339("2026-07-01T11:00:00+08:00")
                    .expect("test start should be valid"),
                end_time: chrono::DateTime::parse_from_rfc3339("2026-08-11T03:59:00+08:00")
                    .expect("test end should be valid"),
                version_id: Some("月之八".to_owned()),
                start_version_id: Some("月之八".to_owned()),
                cover: None,
                labels: Vec::new(),
                source_id: "mys".to_owned(),
                source_news_id: "76308869".to_owned(),
                source_url: "https://example.com/event".to_owned(),
                source_hash: "hash".to_owned(),
            },
            "https://assets.example.com",
        );

        assert_eq!(response.start_time, "2026-07-01T03:00:00Z");
        assert_eq!(response.end_time, "2026-08-10T19:59:00Z");
    }

    #[test]
    fn exports_timed_events_in_utc() {
        let calendar = build_ics(
            "ys",
            "原神",
            &[EventResponse {
                id: "event-1".to_owned(),
                kind: "game_activity".to_owned(),
                title: "幽境危战".to_owned(),
                start_time: "2026-08-19T10:00:00+08:00".to_owned(),
                end_time: "2026-09-22T03:59:00+08:00".to_owned(),
                version: Some("7.0".to_owned()),
                cover: None,
                labels: Vec::new(),
                url: "https://example.com/event".to_owned(),
            }],
            &EventIcsOptions::default(),
        );
        assert!(calendar.contains("DTSTART:20260819T020000Z\r\n"));
        assert!(calendar.contains("DTEND:20260921T195900Z\r\n"));
        assert!(calendar.contains("SUMMARY:幽境危战\r\n"));
    }

    #[test]
    fn exports_start_and_end_milestones_with_reminders() {
        let calendar = build_ics(
            "ys",
            "原神",
            &[EventResponse {
                id: "event-1".to_owned(),
                kind: "game_activity".to_owned(),
                title: "幽境危战".to_owned(),
                start_time: "2026-08-19T10:00:00+08:00".to_owned(),
                end_time: "2026-09-22T03:59:00+08:00".to_owned(),
                version: Some("7.0".to_owned()),
                cover: None,
                labels: Vec::new(),
                url: "https://example.com/event".to_owned(),
            }],
            &EventIcsOptions {
                event_mode: EventIcsMode::Milestones,
                start_reminder_minutes: Some(60),
                end_reminder_minutes: Some(360),
            },
        );

        assert_eq!(calendar.matches("BEGIN:VEVENT").count(), 2);
        assert!(calendar.contains("UID:event-ys-event-1-start@akasha\r\n"));
        assert!(calendar.contains("UID:event-ys-event-1-end@akasha\r\n"));
        assert!(calendar.contains("SUMMARY:【开始】幽境危战\r\n"));
        assert!(calendar.contains("SUMMARY:【结束】幽境危战\r\n"));
        assert!(!calendar.contains("DTEND:"));
        assert!(calendar.contains("TRIGGER;RELATED=START:-PT1H\r\n"));
        assert!(calendar.contains("TRIGGER;RELATED=START:-PT6H\r\n"));
        assert_eq!(calendar.matches("BEGIN:VALARM").count(), 2);
    }

    #[test]
    fn rejects_reminders_over_thirty_days() {
        let options = EventIcsOptions {
            event_mode: EventIcsMode::Span,
            start_reminder_minutes: Some(MAX_REMINDER_MINUTES + 1),
            end_reminder_minutes: None,
        };

        assert!(matches!(options.validate(), Err(AppError::BadRequest(_))));
    }
}
