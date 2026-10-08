use super::super::{china_timezone, endpoints::BirthdayReminderOptions};
use crate::http::error::AppError;
use chrono::{Days, FixedOffset, NaiveDate, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

const DEFAULT_PAST_DAYS: u64 = 30;
const DEFAULT_FUTURE_DAYS: u64 = 366;
const MAX_RANGE_DAYS: i64 = 1_100;
const MAX_REMINDER_MINUTES: u32 = 30 * 24 * 60;

pub(super) const GAME_ACTIVITY: &str = "游戏内活动";
pub(super) const WEB_ACTIVITY: &str = "网页活动";
const VERSION_SCHEDULE: &str = "版本日程";
const BANNER: &str = "卡池";
const BATTLE_PASS: &str = "通行证";
pub(super) const CHARACTER_BIRTHDAY: &str = "角色生日";

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(in crate::features::calendar) struct CalendarSelectorResponse {
    /// 查询参数使用的完整值
    pub(super) value: String,
    /// 前端显示名称
    pub(super) label: String,
    /// 从属于该类型的细分类筛选值
    pub(super) children: Vec<CalendarSelectorOptionResponse>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(in crate::features::calendar) struct CalendarSelectorOptionResponse {
    /// 查询参数使用的完整值
    pub(super) value: String,
    /// 前端显示名称
    pub(super) label: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(in crate::features::calendar) struct CalendarCapabilitiesResponse {
    /// JSON 日程地址
    pub(super) json: String,
    /// ICS 订阅地址
    pub(super) ics: String,
    /// 可用于 include 和 exclude 的完整筛选值
    pub(super) selectors: Vec<CalendarSelectorResponse>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(in crate::features::calendar) struct CalendarQuery {
    /// 查询开始日期，格式为 YYYY-MM-DD，默认包含最近 30 天
    pub(super) from: Option<String>,
    /// 查询结束日期，格式为 YYYY-MM-DD，默认为开始日期后 366 天
    pub(super) to: Option<String>,
    /// 包含的筛选值，可重复；未提供时包含该游戏支持的全部日程
    #[serde(default)]
    pub(super) include: Vec<String>,
    /// 排除的筛选值，可重复，并在 include 之后生效
    #[serde(default)]
    pub(super) exclude: Vec<String>,
    /// JSON 每页数量，默认 100，最大 500；ICS 忽略该参数
    pub(super) limit: Option<u64>,
    /// JSON 分页偏移，默认 0；ICS 忽略该参数
    pub(super) offset: Option<u64>,
}

/// JSON 与 ICS 共用的日程筛选参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[expect(dead_code, reason = "该类型只为 OpenAPI 描述 ICS 的公共筛选参数")]
pub(in crate::features::calendar) struct CalendarFilterParams {
    /// 查询开始日期，格式为 YYYY-MM-DD，默认从今天开始
    pub(super) from: Option<String>,
    /// 查询结束日期，格式为 YYYY-MM-DD
    pub(super) to: Option<String>,
    /// 包含的筛选值，可重复；未提供时包含该游戏支持的全部日程
    #[serde(default)]
    pub(super) include: Vec<String>,
    /// 排除的筛选值，可重复，并在 include 之后生效
    #[serde(default)]
    pub(super) exclude: Vec<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(in crate::features::calendar) struct CalendarIcsOptions {
    /// 时段表示方式，span 为一个连续事件，milestones 为开始和结束两个节点
    #[serde(default)]
    #[param(inline)]
    pub(super) event_mode: CalendarIcsMode,
    /// 普通日程开始前多少分钟提醒，最大 43200 分钟
    pub(super) start_reminder_minutes: Option<u32>,
    /// 普通日程结束前多少分钟提醒，最大 43200 分钟
    pub(super) end_reminder_minutes: Option<u32>,
    /// 生日当天的提醒时间，格式为 HH:MM
    pub(super) birthday_reminder_time: Option<String>,
    /// 生日当天 00:00 前多少分钟提醒，最大 43200 分钟
    pub(super) birthday_reminder_minutes_before: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(in crate::features::calendar) struct CalendarIcsQuery {
    pub(super) from: Option<String>,
    pub(super) to: Option<String>,
    #[serde(default)]
    pub(super) include: Vec<String>,
    #[serde(default)]
    pub(super) exclude: Vec<String>,
    #[serde(default)]
    pub(super) event_mode: CalendarIcsMode,
    pub(super) start_reminder_minutes: Option<u32>,
    pub(super) end_reminder_minutes: Option<u32>,
    pub(super) birthday_reminder_time: Option<String>,
    pub(super) birthday_reminder_minutes_before: Option<u32>,
}

#[derive(Debug, Default, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(super) enum CalendarIcsMode {
    #[default]
    Span,
    Milestones,
}

#[derive(Clone, Copy)]
pub(super) struct DateRange {
    pub(super) from: NaiveDate,
    pub(super) to: NaiveDate,
    pub(super) start: chrono::DateTime<FixedOffset>,
    pub(super) end: chrono::DateTime<FixedOffset>,
}

impl CalendarQuery {
    pub(super) fn time_range(&self, json: bool) -> Result<DateRange, AppError> {
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
    pub(super) fn into_parts(self) -> (CalendarQuery, CalendarIcsOptions) {
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
    pub(super) fn validate(&self) -> Result<(), AppError> {
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

    pub(super) fn birthday_options(&self) -> Result<BirthdayReminderOptions, AppError> {
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
pub(super) struct SelectorFilter {
    pub(super) kinds: Vec<String>,
    pub(super) include: Vec<String>,
    pub(super) exclude: Vec<String>,
}

impl SelectorFilter {
    pub(super) fn new(
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

    pub(super) fn matches(&self, kind: &str, labels: &[String]) -> bool {
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

    pub(super) fn event_kinds(&self) -> Vec<String> {
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

pub(super) fn capabilities(game_id: &str) -> Result<CalendarCapabilitiesResponse, AppError> {
    let (activities, banners): (&[&str], &[&str]) = match game_id {
        "ys" => (&["七圣召唤", "千星奇域", "幻想真境剧诗", "深境螺旋"], &[]),
        "sr" => (&[], &[]),
        "zzz" => (&[], &["代理人调频", "音擎调频"]),
        _ => {
            return Err(AppError::NotFound(format!(
                "calendar is not available for game {game_id}"
            )));
        }
    };
    let selectors = vec![
        selector(GAME_ACTIVITY, activities),
        selector(WEB_ACTIVITY, &[]),
        selector(VERSION_SCHEDULE, &["版本维护", "前瞻特别节目"]),
        selector(BANNER, banners),
        selector(BATTLE_PASS, &[]),
        selector(CHARACTER_BIRTHDAY, &[]),
    ];
    Ok(CalendarCapabilitiesResponse {
        json: format!("/api/v1/games/{game_id}/calendar"),
        ics: format!("/api/v1/games/{game_id}/calendar.ics"),
        selectors,
    })
}

pub(super) fn selector(kind: &str, children: &[&str]) -> CalendarSelectorResponse {
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

pub(super) fn flatten_selector_values(selectors: &[CalendarSelectorResponse]) -> Vec<&str> {
    selectors
        .iter()
        .flat_map(|selector| {
            std::iter::once(selector.value.as_str())
                .chain(selector.children.iter().map(|child| child.value.as_str()))
        })
        .collect()
}

pub(super) fn parse_date(value: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("calendar dates must use YYYY-MM-DD".to_owned()))
}
