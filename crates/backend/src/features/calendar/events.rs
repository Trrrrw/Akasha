mod query;
use query::*;

use akasha_application::calendar::{CalendarEvent, ListCalendarEventsFilter};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Query as MultiQuery;
use chrono::{Datelike, NaiveDate, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use super::{
    endpoints::{CharacterBirthdayResponse, append_birthdays_to_ics, list_birthdays},
    ics::{AlarmOffset, AlarmRelation, IcsCalendar, IcsEvent},
};
use crate::{
    http::{
        error::AppError,
        path::{GamePath, require_game},
        response::{ErrorResponse, PageResponse, public_asset_url, utc_timestamp},
    },
    state::AppState,
};

const INTERNAL_ENTRY_LIMIT: u64 = 10_000;
const DEFAULT_JSON_LIMIT: u64 = 100;
const MAX_JSON_LIMIT: u64 = 500;
#[derive(Debug, Clone, Serialize, ToSchema)]
pub(super) struct CalendarEntryResponse {
    id: String,
    kind: String,
    title: String,
    /// 全天日程使用 YYYY-MM-DD，定时日程使用 UTC RFC 3339
    start: String,
    /// 全天日程为不包含在事件内的结束日期，定时日程使用 UTC RFC 3339
    end: String,
    all_day: bool,
    version: Option<String>,
    cover: Option<String>,
    labels: Vec<String>,
    url: String,
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/calendar/capabilities",
    tag = "Calendar",
    summary = "获取日程筛选规则",
    description = "返回指定游戏支持的 JSON 与 ICS 输出地址，以及 include 和 exclude 可使用的完整筛选值",
    params(GamePath),
    responses(
        (status = 200, body = CalendarCapabilitiesResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn calendar_capabilities(
    Path(GamePath { game_id }): Path<GamePath>,
    State(state): State<AppState>,
) -> Result<Json<CalendarCapabilitiesResponse>, AppError> {
    require_game(&state, &game_id).await?;
    Ok(Json(capabilities(&game_id)?))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/calendar",
    tag = "Calendar",
    summary = "获取游戏日程 JSON",
    description = "分页返回游戏日程，包括活动、固定周期常驻玩法、版本日程、卡池、通行证和角色生日",
    params(GamePath, CalendarQuery),
    responses(
        (status = 200, body = PageResponse<CalendarEntryResponse>),
        (status = 400, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn calendar_json(
    Path(GamePath { game_id }): Path<GamePath>,
    State(state): State<AppState>,
    MultiQuery(query): MultiQuery<CalendarQuery>,
) -> Result<Json<PageResponse<CalendarEntryResponse>>, AppError> {
    require_game(&state, &game_id).await?;
    let capability = capabilities(&game_id)?;
    let range = query.time_range(true)?;
    let filters = SelectorFilter::new(&query.include, &query.exclude, &capability)?;
    let limit = query.limit.unwrap_or(DEFAULT_JSON_LIMIT);
    let offset = query.offset.unwrap_or(0);
    if limit == 0 || limit > MAX_JSON_LIMIT {
        return Err(AppError::BadRequest(
            "calendar limit must be between 1 and 500".to_owned(),
        ));
    }

    let mut items = list_event_entries(&state, &game_id, range, &filters).await?;
    if filters.matches(CHARACTER_BIRTHDAY, &[]) {
        let birthday_items = list_birthdays(&state, &game_id).await?;
        items.extend(materialize_birthdays(
            &game_id,
            &birthday_items,
            range,
            &state.config().asset_base_url,
        ));
    }
    items.sort_by(|left, right| left.start.cmp(&right.start).then(left.id.cmp(&right.id)));
    let total = items.len() as u64;
    let items = items
        .into_iter()
        .skip(usize::try_from(offset).unwrap_or(usize::MAX))
        .take(limit as usize)
        .collect();
    Ok(Json(PageResponse {
        total,
        limit,
        offset,
        items,
        meta: (),
    }))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/calendar.ics",
    tag = "Calendar",
    summary = "获取游戏日程 ICS",
    description = "导出筛选后的游戏日程；默认只包含今天起仍有效的内容，角色生日以每年重复事件表示",
    params(GamePath, CalendarFilterParams, CalendarIcsOptions),
    responses(
        (status = 200, content_type = "text/calendar", body = String),
        (status = 400, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn calendar_ics(
    Path(GamePath { game_id }): Path<GamePath>,
    State(state): State<AppState>,
    MultiQuery(query): MultiQuery<CalendarIcsQuery>,
) -> Result<Response, AppError> {
    require_game(&state, &game_id).await?;
    let capability = capabilities(&game_id)?;
    let (filter_query, options) = query.into_parts();
    options.validate()?;
    let range = filter_query.time_range(false)?;
    let filters = SelectorFilter::new(&filter_query.include, &filter_query.exclude, &capability)?;
    let items = list_event_entries(&state, &game_id, range, &filters).await?;
    let game = state
        .application()
        .find_game(&game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("game {game_id} not found")))?;
    let mut calendar = IcsCalendar::new(
        "-//Akasha//Game Calendar//ZH-CN",
        &format!("{}日程", game.name_zh),
    );
    append_timed_to_ics(&mut calendar, &game_id, &items, &options);
    if filters.matches(CHARACTER_BIRTHDAY, &[]) {
        let birthday_items = list_birthdays(&state, &game_id)
            .await?
            .into_iter()
            .filter(|item| birthday_occurs_in_range(item, range))
            .collect::<Vec<_>>();
        append_birthdays_to_ics(
            &mut calendar,
            &game_id,
            &birthday_items,
            &options.birthday_options()?,
        );
    }
    let disposition =
        HeaderValue::from_str(&format!("attachment; filename=\"{game_id}-calendar.ics\""))
            .map_err(|error| AppError::Internal(error.into()))?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/calendar; charset=utf-8"),
            ),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        calendar.finish(),
    )
        .into_response())
}

async fn list_event_entries(
    state: &AppState,
    game_id: &str,
    range: DateRange,
    filters: &SelectorFilter,
) -> Result<Vec<CalendarEntryResponse>, AppError> {
    let kinds = filters.event_kinds();
    if kinds.is_empty() {
        return Ok(Vec::new());
    }
    let rows = state
        .application()
        .list_calendar_events(ListCalendarEventsFilter {
            game_id: game_id.to_owned(),
            start_time: range.start,
            end_time: range.end,
            kinds,
            limit: INTERNAL_ENTRY_LIMIT,
        })
        .await?;
    Ok(rows
        .into_iter()
        .filter(|event| filters.matches(&event.kind, &event.labels))
        .map(|event| CalendarEntryResponse::from_event(event, &state.config().asset_base_url))
        .collect())
}

fn materialize_birthdays(
    game_id: &str,
    birthdays: &[CharacterBirthdayResponse],
    range: DateRange,
    asset_base_url: &str,
) -> Vec<CalendarEntryResponse> {
    let mut items = Vec::new();
    for year in range.from.year()..=range.to.year() {
        for item in birthdays {
            let Some(date) = birthday_date(year, item.birthday_month, item.birthday_day) else {
                continue;
            };
            if date < range.from || date >= range.to {
                continue;
            }
            let end = date
                .succ_opt()
                .expect("birthday date should have a next day");
            items.push(CalendarEntryResponse {
                id: format!("character-{}-{year}", item.character_id),
                kind: CHARACTER_BIRTHDAY.to_owned(),
                title: format!("{}生日", item.character_name),
                start: date.format("%Y-%m-%d").to_string(),
                end: end.format("%Y-%m-%d").to_string(),
                all_day: true,
                version: None,
                cover: item.character_icon.clone(),
                labels: Vec::new(),
                url: format!(
                    "{}/api/v1/games/{game_id}/data/character/{}",
                    asset_base_url.trim_end_matches('/'),
                    item.character_id
                ),
            });
        }
    }
    items
}

fn birthday_date(year: i32, month: i16, day: i16) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month as u32, day as u32).or_else(|| {
        // JSON 直接物化每年的日期，因此可以在平年精确回退到二月最后一天
        (month == 2 && day == 29)
            .then(|| NaiveDate::from_ymd_opt(year, 2, 28).expect("February 28 should be valid"))
    })
}

fn birthday_occurs_in_range(item: &CharacterBirthdayResponse, range: DateRange) -> bool {
    (range.from.year()..=range.to.year()).any(|year| {
        birthday_date(year, item.birthday_month, item.birthday_day)
            .is_some_and(|date| date >= range.from && date < range.to)
    })
}

impl CalendarEntryResponse {
    fn from_event(event: CalendarEvent, asset_base_url: &str) -> Self {
        Self {
            id: event.id,
            kind: event.kind,
            title: event.title,
            start: utc_timestamp(event.start_time),
            end: utc_timestamp(event.end_time),
            all_day: false,
            version: event.version_id,
            cover: public_asset_url(asset_base_url, event.cover),
            labels: event.labels,
            url: event.source_url,
        }
    }
}

fn append_timed_to_ics(
    calendar: &mut IcsCalendar,
    game_id: &str,
    items: &[CalendarEntryResponse],
    options: &CalendarIcsOptions,
) {
    for item in items {
        let start = chrono::DateTime::parse_from_rfc3339(&item.start)
            .expect("stored calendar start should be RFC 3339")
            .with_timezone(&Utc);
        let end = chrono::DateTime::parse_from_rfc3339(&item.end)
            .expect("stored calendar end should be RFC 3339")
            .with_timezone(&Utc);
        match options.event_mode {
            CalendarIcsMode::Span => {
                let mut event = IcsEvent::new(&format!("calendar-{game_id}-{}@akasha", item.id))
                    .starts_at(start)
                    .ends_at(end)
                    .summary(&item.title)
                    .url(&item.url)
                    .transparent();
                if let Some(minutes) = options.start_reminder_minutes {
                    event = event.display_alarm(
                        AlarmRelation::Start,
                        AlarmOffset::Before(minutes),
                        &format!("日程即将开始：{}", item.title),
                    );
                }
                if let Some(minutes) = options.end_reminder_minutes {
                    event = event.display_alarm(
                        AlarmRelation::End,
                        AlarmOffset::Before(minutes),
                        &format!("日程即将结束：{}", item.title),
                    );
                }
                calendar.push_event(event);
            }
            CalendarIcsMode::Milestones => {
                let mut start_event =
                    IcsEvent::new(&format!("calendar-{game_id}-{}-start@akasha", item.id))
                        .starts_at(start)
                        .summary(&format!("【开始】{}", item.title))
                        .url(&item.url)
                        .transparent();
                if let Some(minutes) = options.start_reminder_minutes {
                    start_event = start_event.display_alarm(
                        AlarmRelation::Start,
                        AlarmOffset::Before(minutes),
                        &format!("日程即将开始：{}", item.title),
                    );
                }
                calendar.push_event(start_event);
                let mut end_event =
                    IcsEvent::new(&format!("calendar-{game_id}-{}-end@akasha", item.id))
                        .starts_at(end)
                        .summary(&format!("【结束】{}", item.title))
                        .url(&item.url)
                        .transparent();
                if let Some(minutes) = options.end_reminder_minutes {
                    end_event = end_event.display_alarm(
                        AlarmRelation::Start,
                        AlarmOffset::Before(minutes),
                        &format!("日程即将结束：{}", item.title),
                    );
                }
                calendar.push_event(end_event);
            }
        }
    }
}

#[cfg(test)]
mod tests;
