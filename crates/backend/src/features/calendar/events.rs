use akasha_application::calendar::{CalendarEvent, ListCalendarEventsFilter};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Query as MultiQuery;
use chrono::{Datelike, Days, FixedOffset, NaiveDate, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{
    china_timezone,
    endpoints::{
        BirthdayReminderOptions, CharacterBirthdayResponse, append_birthdays_to_ics, list_birthdays,
    },
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
const DEFAULT_PAST_DAYS: u64 = 30;
const DEFAULT_FUTURE_DAYS: u64 = 366;
const MAX_RANGE_DAYS: i64 = 1_100;
const MAX_REMINDER_MINUTES: u32 = 30 * 24 * 60;

const GAME_ACTIVITY: &str = "游戏内活动";
const WEB_ACTIVITY: &str = "网页活动";
const VERSION_SCHEDULE: &str = "版本日程";
const BANNER: &str = "卡池";
const BATTLE_PASS: &str = "通行证";
const CHARACTER_BIRTHDAY: &str = "角色生日";

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(super) struct CalendarSelectorResponse {
    /// 查询参数使用的完整值
    value: String,
    /// 前端显示名称
    label: String,
    /// 从属于该类型的细分类筛选值
    children: Vec<CalendarSelectorOptionResponse>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(super) struct CalendarSelectorOptionResponse {
    /// 查询参数使用的完整值
    value: String,
    /// 前端显示名称
    label: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(super) struct CalendarCapabilitiesResponse {
    /// JSON 日程地址
    json: String,
    /// ICS 订阅地址
    ics: String,
    /// 可用于 include 和 exclude 的完整筛选值
    selectors: Vec<CalendarSelectorResponse>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct CalendarQuery {
    /// 查询开始日期，格式为 YYYY-MM-DD；JSON 默认包含最近 30 天，ICS 默认从今天开始
    from: Option<String>,
    /// 查询结束日期，格式为 YYYY-MM-DD，默认为开始日期后 366 天
    to: Option<String>,
    /// 包含的筛选值，可重复；未提供时包含该游戏支持的全部日程
    #[serde(default)]
    include: Vec<String>,
    /// 排除的筛选值，可重复，并在 include 之后生效
    #[serde(default)]
    exclude: Vec<String>,
    /// JSON 每页数量，默认 100，最大 500；ICS 忽略该参数
    limit: Option<u64>,
    /// JSON 分页偏移，默认 0；ICS 忽略该参数
    offset: Option<u64>,
}

/// JSON 与 ICS 共用的日程筛选参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[expect(dead_code, reason = "该类型只为 OpenAPI 描述 ICS 的公共筛选参数")]
pub(super) struct CalendarFilterParams {
    /// 查询开始日期，格式为 YYYY-MM-DD
    from: Option<String>,
    /// 查询结束日期，格式为 YYYY-MM-DD
    to: Option<String>,
    /// 包含的筛选值，可重复；未提供时包含该游戏支持的全部日程
    #[serde(default)]
    include: Vec<String>,
    /// 排除的筛选值，可重复，并在 include 之后生效
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct CalendarIcsOptions {
    /// 时段表示方式，span 为一个连续事件，milestones 为开始和结束两个节点
    #[serde(default)]
    #[param(inline)]
    event_mode: CalendarIcsMode,
    /// 普通日程开始前多少分钟提醒，最大 43200 分钟
    start_reminder_minutes: Option<u32>,
    /// 普通日程结束前多少分钟提醒，最大 43200 分钟
    end_reminder_minutes: Option<u32>,
    /// 生日当天的提醒时间，格式为 HH:MM
    birthday_reminder_time: Option<String>,
    /// 生日当天 00:00 前多少分钟提醒，最大 43200 分钟
    birthday_reminder_minutes_before: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(super) struct CalendarIcsQuery {
    from: Option<String>,
    to: Option<String>,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    event_mode: CalendarIcsMode,
    start_reminder_minutes: Option<u32>,
    end_reminder_minutes: Option<u32>,
    birthday_reminder_time: Option<String>,
    birthday_reminder_minutes_before: Option<u32>,
}

#[derive(Debug, Default, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum CalendarIcsMode {
    #[default]
    Span,
    Milestones,
}

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
    description = "分页返回服务器中可表示为日程的数据，包括活动、版本日程、卡池、通行证和角色生日",
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

    let mut items = list_persisted_entries(&state, &game_id, range, &filters).await?;
    if filters.matches(CHARACTER_BIRTHDAY, &[]) {
        let birthday_items = birthdays(&state, &game_id).await?;
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
    let items = list_persisted_entries(&state, &game_id, range, &filters).await?;
    let game = state
        .application()
        .find_game(&game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("game {game_id} not found")))?;
    let mut calendar = IcsCalendar::new(
        "-//Akasha//Game Calendar//ZH-CN",
        &format!("{}日程", game.name_zh),
    );
    append_persisted_to_ics(&mut calendar, &game_id, &items, &options);
    if filters.matches(CHARACTER_BIRTHDAY, &[]) {
        let birthday_items = birthdays(&state, &game_id)
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

#[derive(Clone, Copy)]
struct DateRange {
    from: NaiveDate,
    to: NaiveDate,
    start: chrono::DateTime<FixedOffset>,
    end: chrono::DateTime<FixedOffset>,
}

impl CalendarQuery {
    fn time_range(&self, json: bool) -> Result<DateRange, AppError> {
        let timezone = china_timezone();
        let today = Utc::now().with_timezone(&timezone).date_naive();
        let default_from = if json {
            today
                .checked_sub_days(Days::new(DEFAULT_PAST_DAYS))
                .unwrap_or(today)
        } else {
            today
        };
        let from = self
            .from
            .as_deref()
            .map(parse_date)
            .transpose()?
            .unwrap_or(default_from);
        let to = self
            .to
            .as_deref()
            .map(parse_date)
            .transpose()?
            .unwrap_or_else(|| {
                from.checked_add_days(Days::new(DEFAULT_FUTURE_DAYS))
                    .unwrap_or(from)
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
        Ok(DateRange {
            from,
            to,
            start,
            end,
        })
    }
}

impl CalendarIcsQuery {
    fn into_parts(self) -> (CalendarQuery, CalendarIcsOptions) {
        (
            CalendarQuery {
                from: self.from,
                to: self.to,
                include: self.include,
                exclude: self.exclude,
                limit: None,
                offset: None,
            },
            CalendarIcsOptions {
                event_mode: self.event_mode,
                start_reminder_minutes: self.start_reminder_minutes,
                end_reminder_minutes: self.end_reminder_minutes,
                birthday_reminder_time: self.birthday_reminder_time,
                birthday_reminder_minutes_before: self.birthday_reminder_minutes_before,
            },
        )
    }
}

impl CalendarIcsOptions {
    fn validate(&self) -> Result<(), AppError> {
        if self
            .start_reminder_minutes
            .is_some_and(|value| value > MAX_REMINDER_MINUTES)
            || self
                .end_reminder_minutes
                .is_some_and(|value| value > MAX_REMINDER_MINUTES)
            || self
                .birthday_reminder_minutes_before
                .is_some_and(|value| value > MAX_REMINDER_MINUTES)
        {
            return Err(AppError::BadRequest(
                "calendar reminders must be between 0 and 43200 minutes".to_owned(),
            ));
        }
        let _ = self.birthday_options()?;
        Ok(())
    }

    fn birthday_options(&self) -> Result<BirthdayReminderOptions, AppError> {
        let reminder_time_minutes = self
            .birthday_reminder_time
            .as_deref()
            .map(str::trim)
            .map(|value| {
                let time = chrono::NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| {
                    AppError::BadRequest("birthday_reminder_time must use HH:MM".to_owned())
                })?;
                Ok::<u32, AppError>(time.hour() * 60 + time.minute())
            })
            .transpose()?;
        Ok(BirthdayReminderOptions {
            reminder_time_minutes,
            reminder_minutes_before: self.birthday_reminder_minutes_before,
        })
    }
}

#[derive(Debug)]
struct SelectorFilter {
    kinds: Vec<String>,
    include: Vec<String>,
    exclude: Vec<String>,
}

impl SelectorFilter {
    fn new(
        include: &[String],
        exclude: &[String],
        capability: &CalendarCapabilitiesResponse,
    ) -> Result<Self, AppError> {
        let valid = flatten_selector_values(&capability.selectors);
        let normalize = |values: &[String]| -> Result<Vec<String>, AppError> {
            let mut result = Vec::new();
            for value in values {
                let value = value.trim();
                if value.is_empty() || result.iter().any(|existing| existing == value) {
                    continue;
                }
                if !valid.contains(&value) {
                    return Err(AppError::BadRequest(format!(
                        "unsupported calendar selector: {value}"
                    )));
                }
                result.push(value.to_owned());
            }
            Ok(result)
        };
        Ok(Self {
            kinds: capability
                .selectors
                .iter()
                .map(|selector| selector.value.clone())
                .collect(),
            include: normalize(include)?,
            exclude: normalize(exclude)?,
        })
    }

    fn matches(&self, kind: &str, labels: &[String]) -> bool {
        if !self.kinds.iter().any(|value| value == kind) {
            return false;
        }
        let selector_matches = |selector: &str| {
            selector == kind
                || selector
                    .strip_prefix(&format!("{kind}:"))
                    .is_some_and(|label| labels.iter().any(|value| value == label))
        };
        (self.include.is_empty() || self.include.iter().any(|value| selector_matches(value)))
            && !self.exclude.iter().any(|value| selector_matches(value))
    }

    fn persisted_kinds(&self) -> Vec<String> {
        self.kinds
            .iter()
            .filter(|kind| kind.as_str() != CHARACTER_BIRTHDAY)
            .filter(|kind| {
                self.include.is_empty()
                    || self.include.iter().any(|selector| {
                        selector == kind.as_str() || selector.starts_with(&format!("{kind}:"))
                    })
            })
            .filter(|kind| {
                !self
                    .exclude
                    .iter()
                    .any(|selector| selector == kind.as_str())
            })
            .cloned()
            .collect()
    }
}

fn capabilities(game_id: &str) -> Result<CalendarCapabilitiesResponse, AppError> {
    let selectors = match game_id {
        "ys" => vec![
            selector(GAME_ACTIVITY, &["七圣召唤", "千星奇域"]),
            selector(WEB_ACTIVITY, &[]),
            selector(VERSION_SCHEDULE, &["版本维护", "前瞻特别节目"]),
            selector(BANNER, &[]),
            selector(BATTLE_PASS, &[]),
            selector(CHARACTER_BIRTHDAY, &[]),
        ],
        "sr" => vec![
            selector(GAME_ACTIVITY, &[]),
            selector(WEB_ACTIVITY, &[]),
            selector(VERSION_SCHEDULE, &["版本维护", "前瞻特别节目"]),
            selector(BANNER, &[]),
            selector(BATTLE_PASS, &[]),
            selector(CHARACTER_BIRTHDAY, &[]),
        ],
        "zzz" => vec![
            selector(GAME_ACTIVITY, &[]),
            selector(WEB_ACTIVITY, &[]),
            selector(VERSION_SCHEDULE, &["版本维护", "前瞻特别节目"]),
            selector(BANNER, &["代理人调频", "音擎调频"]),
            selector(BATTLE_PASS, &[]),
            selector(CHARACTER_BIRTHDAY, &[]),
        ],
        _ => {
            return Err(AppError::NotFound(format!(
                "calendar is not available for game {game_id}"
            )));
        }
    };
    Ok(CalendarCapabilitiesResponse {
        json: format!("/api/v1/games/{game_id}/calendar"),
        ics: format!("/api/v1/games/{game_id}/calendar.ics"),
        selectors,
    })
}

fn selector(kind: &str, children: &[&str]) -> CalendarSelectorResponse {
    CalendarSelectorResponse {
        value: kind.to_owned(),
        label: kind.to_owned(),
        children: children
            .iter()
            .map(|label| CalendarSelectorOptionResponse {
                value: format!("{kind}:{label}"),
                label: (*label).to_owned(),
            })
            .collect(),
    }
}

fn flatten_selector_values(selectors: &[CalendarSelectorResponse]) -> Vec<&str> {
    selectors
        .iter()
        .flat_map(|selector| {
            std::iter::once(selector.value.as_str())
                .chain(selector.children.iter().map(|child| child.value.as_str()))
        })
        .collect()
}

async fn list_persisted_entries(
    state: &AppState,
    game_id: &str,
    range: DateRange,
    filters: &SelectorFilter,
) -> Result<Vec<CalendarEntryResponse>, AppError> {
    let kinds = filters.persisted_kinds();
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

async fn birthdays(
    state: &AppState,
    game_id: &str,
) -> Result<Vec<CharacterBirthdayResponse>, AppError> {
    list_birthdays(state, game_id).await
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

fn append_persisted_to_ics(
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

fn parse_date(value: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("calendar dates must use YYYY-MM-DD".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repeated_ics_selectors() {
        let uri = "/calendar.ics?include=版本日程&include=角色生日&exclude=版本日程%3A版本维护"
            .parse()
            .expect("test URI should be valid");
        let MultiQuery(query) = MultiQuery::<CalendarIcsQuery>::try_from_uri(&uri)
            .expect("ICS query should deserialize");
        let (filter, _) = query.into_parts();
        assert_eq!(filter.include, ["版本日程", "角色生日"]);
        assert_eq!(filter.exclude, ["版本日程:版本维护"]);
    }

    #[test]
    fn exposes_complete_selector_values() {
        let capability = capabilities("ys").expect("ys should support calendar");
        let values = flatten_selector_values(&capability.selectors);
        assert!(values.contains(&"游戏内活动"));
        assert!(values.contains(&"游戏内活动:七圣召唤"));
        assert!(values.contains(&"版本日程:前瞻特别节目"));
        assert!(values.contains(&"角色生日"));
    }

    #[test]
    fn exposes_zzz_calendar_selectors() {
        let capability = capabilities("zzz").expect("zzz should support calendar");
        let values = flatten_selector_values(&capability.selectors);
        assert!(values.contains(&"游戏内活动"));
        assert!(values.contains(&"网页活动"));
        assert!(values.contains(&"版本日程:版本维护"));
        assert!(values.contains(&"卡池:代理人调频"));
        assert!(values.contains(&"卡池:音擎调频"));
        assert!(values.contains(&"通行证"));
        assert!(values.contains(&"角色生日"));
    }

    #[test]
    fn applies_include_then_exclude() {
        let capability = capabilities("ys").expect("ys should support calendar");
        let filter = SelectorFilter::new(
            &[GAME_ACTIVITY.to_owned()],
            &["游戏内活动:七圣召唤".to_owned()],
            &capability,
        )
        .expect("selectors should be valid");
        assert!(filter.matches(GAME_ACTIVITY, &[]));
        assert!(!filter.matches(GAME_ACTIVITY, &["七圣召唤".to_owned()]));
        assert!(!filter.matches(WEB_ACTIVITY, &[]));
        assert_eq!(filter.persisted_kinds(), [GAME_ACTIVITY]);
    }

    #[test]
    fn materializes_leap_day_on_february_last_day() {
        assert_eq!(
            birthday_date(2025, 2, 29),
            NaiveDate::from_ymd_opt(2025, 2, 28)
        );
        assert_eq!(
            birthday_date(2028, 2, 29),
            NaiveDate::from_ymd_opt(2028, 2, 29)
        );
    }

    #[test]
    fn rejects_unknown_selectors() {
        let capability = capabilities("sr").expect("sr should support calendar");
        assert!(matches!(
            SelectorFilter::new(&["活动".to_owned()], &[], &capability),
            Err(AppError::BadRequest(_))
        ));
    }
}
