use akasha_application::{
    characters::{SrCharacterListFilter, YsCharacterListFilter, ZzzCharacterListFilter},
    search::TextQuery,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use chrono::{NaiveDate, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::ics::{AlarmOffset, AlarmRelation, IcsCalendar, IcsEvent};

use crate::{
    http::{
        error::AppError,
        path::{GamePath, require_game},
        response::{ErrorResponse, ListResponse, public_asset_url},
    },
    state::AppState,
};

const CHARACTER_LIMIT: u64 = 1_000;
const MAX_BIRTHDAY_REMINDER_MINUTES: u32 = 30 * 24 * 60;

/// 角色生日日历查询参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct CharacterBirthdayQuery {
    /// 角色名称和简介查询
    q: Option<String>,
    /// 生日月份，取值 1 到 12
    birthday_month: Option<i16>,
    /// 性别，仅支持 male 或 female，当前仅绝区零提供该字段
    gender: Option<String>,
}

/// 角色生日 ICS 查询参数
#[derive(Debug, Deserialize)]
pub(super) struct CharacterBirthdayIcsQuery {
    q: Option<String>,
    birthday_month: Option<i16>,
    gender: Option<String>,
    reminder_time: Option<String>,
    reminder_minutes_before: Option<u32>,
}

/// 角色生日 ICS 提醒选项
#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct CharacterBirthdayIcsOptions {
    /// 生日当天的提醒时间，格式为 HH:MM，使用用户日历所在时区
    reminder_time: Option<String>,
    /// 生日当天 00:00 前多少分钟提醒，最大 43200 分钟
    reminder_minutes_before: Option<u32>,
}

#[derive(Debug, Default)]
struct BirthdayReminderOptions {
    reminder_time_minutes: Option<u32>,
    reminder_minutes_before: Option<u32>,
}

/// 角色生日条目
#[derive(Debug, Serialize, ToSchema)]
pub(super) struct CharacterBirthdayResponse {
    character_id: String,
    character_name: String,
    character_icon: Option<String>,
    birthday_month: i16,
    birthday_day: i16,
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/calendar/character-birthdays",
    tag = "Calendar",
    summary = "获取角色生日 JSON",
    description = "以 JSON 格式返回拥有生日数据的游戏角色，可按名称、月份和接口提供的角色字段筛选",
    params(GamePath, CharacterBirthdayQuery),
    responses(
        (status = 200, body = ListResponse<CharacterBirthdayResponse>),
        (status = 400, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn character_birthdays_json(
    Path(GamePath { game_id }): Path<GamePath>,
    State(state): State<AppState>,
    Query(query): Query<CharacterBirthdayQuery>,
) -> Result<Json<ListResponse<CharacterBirthdayResponse>>, AppError> {
    let items = list_birthdays(&state, &game_id, query).await?;
    Ok(Json(ListResponse {
        total: items.len() as u64,
        items,
    }))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/calendar/character-birthdays.ics",
    tag = "Calendar",
    summary = "获取角色生日 ICS",
    description = "按游戏导出每年重复的角色生日事件，可按名称和月份筛选，并设置生日当天或提前提醒",
    params(GamePath, CharacterBirthdayQuery, CharacterBirthdayIcsOptions),
    responses(
        (status = 200, content_type = "text/calendar", body = String),
        (status = 400, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn character_birthdays_ics(
    Path(GamePath { game_id }): Path<GamePath>,
    State(state): State<AppState>,
    Query(query): Query<CharacterBirthdayIcsQuery>,
) -> Result<Response, AppError> {
    let (filter, reminder_options) = query.into_parts();
    let reminder_options = reminder_options.validate()?;
    let items = list_birthdays(&state, &game_id, filter).await?;
    let game = state
        .application()
        .find_game(&game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("game {game_id} not found")))?;
    let calendar = build_ics(&game_id, &game.name_zh, &items, &reminder_options);
    let filename = format!("{game_id}-character-birthdays.ics");
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
        calendar,
    )
        .into_response())
}

async fn list_birthdays(
    state: &AppState,
    game_id: &str,
    query: CharacterBirthdayQuery,
) -> Result<Vec<CharacterBirthdayResponse>, AppError> {
    require_game(state, game_id).await?;
    validate_calendar_game(game_id)?;

    if let Some(month) = query.birthday_month
        && !(1..=12).contains(&month)
    {
        return Err(AppError::BadRequest(
            "birthday_month must be between 1 and 12".to_owned(),
        ));
    }
    let gender = query
        .gender
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if !matches!(gender.as_deref(), None | Some("male" | "female")) {
        return Err(AppError::BadRequest(
            "gender must be male or female".to_owned(),
        ));
    }
    if gender.is_some() && game_id != "zzz" {
        return Err(AppError::BadRequest(
            "gender is not available for this game's character data".to_owned(),
        ));
    }
    let text_query = query
        .q
        .as_deref()
        .map(TextQuery::parse)
        .transpose()
        .map_err(|error| AppError::BadRequest(error.to_string()))?
        .filter(|query| !query.is_empty());
    let asset_base_url = &state.config().asset_base_url;

    let items = match game_id {
        "ys" => state
            .application()
            .list_ys_characters(YsCharacterListFilter {
                query: text_query,
                element: None,
                weapon_type: None,
                rarity: None,
                region: None,
                affiliation: None,
                voice_actor: None,
                birthday_month: query.birthday_month,
                birthday_day: None,
                special: None,
                birthday_only: true,
                limit: CHARACTER_LIMIT,
                offset: 0,
            })
            .await?
            .1
            .into_iter()
            .filter_map(|item| {
                birthday(
                    item.id,
                    item.name,
                    item.icon_url,
                    item.birthday_month,
                    item.birthday_day,
                    asset_base_url,
                )
            })
            .collect(),
        "sr" => state
            .application()
            .list_sr_characters(SrCharacterListFilter {
                query: text_query,
                path: None,
                combat_type: None,
                rarity: None,
                camp: None,
                voice_actor: None,
                birthday_month: query.birthday_month,
                birthday_day: None,
                birthday_only: true,
                limit: CHARACTER_LIMIT,
                offset: 0,
            })
            .await?
            .1
            .into_iter()
            .filter_map(|item| {
                birthday(
                    item.id,
                    item.name,
                    item.icon_url,
                    item.birthday_month,
                    item.birthday_day,
                    asset_base_url,
                )
            })
            .collect(),
        "zzz" => state
            .application()
            .list_zzz_characters(ZzzCharacterListFilter {
                query: text_query,
                specialty_id: None,
                specialty: None,
                element_id: None,
                element: None,
                hit_type_id: None,
                hit_type: None,
                camp_id: None,
                camp: None,
                rarity: None,
                gender,
                special_element: None,
                birthday_month: query.birthday_month,
                birthday_day: None,
                birthday_only: true,
                limit: CHARACTER_LIMIT,
                offset: 0,
            })
            .await?
            .1
            .into_iter()
            .filter_map(|item| {
                birthday(
                    item.id,
                    item.name,
                    item.icon_url,
                    item.birthday_month,
                    item.birthday_day,
                    asset_base_url,
                )
            })
            .collect(),
        _ => {
            return Err(AppError::NotFound(format!(
                "character calendar is not available for game {game_id}"
            )));
        }
    };
    Ok(items)
}

fn validate_calendar_game(game_id: &str) -> Result<(), AppError> {
    if matches!(game_id, "ys" | "sr" | "zzz") {
        Ok(())
    } else {
        Err(AppError::NotFound(format!(
            "character calendar is not available for game {game_id}"
        )))
    }
}

fn birthday(
    id: String,
    name: String,
    icon: String,
    month: Option<i16>,
    day: Option<i16>,
    asset_base_url: &str,
) -> Option<CharacterBirthdayResponse> {
    Some(CharacterBirthdayResponse {
        character_id: id,
        character_name: name,
        character_icon: icon
            .starts_with("/assets/game-data/")
            .then_some(icon)
            .and_then(|value| public_asset_url(asset_base_url, Some(value))),
        birthday_month: month?,
        birthday_day: day?,
    })
}

impl CharacterBirthdayIcsQuery {
    fn into_parts(self) -> (CharacterBirthdayQuery, CharacterBirthdayIcsOptions) {
        (
            CharacterBirthdayQuery {
                q: self.q,
                birthday_month: self.birthday_month,
                gender: self.gender,
            },
            CharacterBirthdayIcsOptions {
                reminder_time: self.reminder_time,
                reminder_minutes_before: self.reminder_minutes_before,
            },
        )
    }
}

impl CharacterBirthdayIcsOptions {
    fn validate(&self) -> Result<BirthdayReminderOptions, AppError> {
        let reminder_time_minutes = self
            .reminder_time
            .as_deref()
            .map(str::trim)
            .map(|value| {
                if value.len() != 5 {
                    return Err(AppError::BadRequest(
                        "birthday reminder_time must use HH:MM".to_owned(),
                    ));
                }
                NaiveTime::parse_from_str(value, "%H:%M")
                    .map(|time| time.hour() * 60 + time.minute())
                    .map_err(|_| {
                        AppError::BadRequest("birthday reminder_time must use HH:MM".to_owned())
                    })
            })
            .transpose()?;
        if self
            .reminder_minutes_before
            .is_some_and(|minutes| minutes > MAX_BIRTHDAY_REMINDER_MINUTES)
        {
            return Err(AppError::BadRequest(
                "birthday reminder_minutes_before must be between 0 and 43200".to_owned(),
            ));
        }
        Ok(BirthdayReminderOptions {
            reminder_time_minutes,
            reminder_minutes_before: self.reminder_minutes_before,
        })
    }
}

fn build_ics(
    game_id: &str,
    game_name: &str,
    items: &[CharacterBirthdayResponse],
    reminder_options: &BirthdayReminderOptions,
) -> String {
    let calendar_name = format!("{game_name}角色生日");
    let mut calendar = IcsCalendar::new("-//Akasha//Character Birthdays//ZH-CN", &calendar_name);
    for item in items {
        let date =
            NaiveDate::from_ymd_opt(2000, item.birthday_month as u32, item.birthday_day as u32)
                .expect("stored birthday should be a valid date in leap year 2000");
        let mut event = IcsEvent::new(&format!("character-{game_id}-{}@akasha", item.character_id))
            .starts_on(date)
            .recurrence(birthday_recurrence(item.birthday_month, item.birthday_day))
            .summary(&format!("{}生日", item.character_name))
            .transparent();
        if let Some(minutes) = reminder_options.reminder_minutes_before {
            event = event.display_alarm(
                AlarmRelation::Start,
                AlarmOffset::Before(minutes),
                &format!("{}的生日即将到来", item.character_name),
            );
        }
        if let Some(minutes) = reminder_options.reminder_time_minutes
            && !(minutes == 0 && reminder_options.reminder_minutes_before == Some(0))
        {
            event = event.display_alarm(
                AlarmRelation::Start,
                AlarmOffset::After(minutes),
                &format!("今天是{}的生日", item.character_name),
            );
        }
        calendar.push_event(event);
    }
    calendar.finish()
}

/// 让 2 月 29 日生日在非闰年回退到二月最后一天
fn birthday_recurrence(month: i16, day: i16) -> &'static str {
    if month == 2 && day == 29 {
        "FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=-1"
    } else {
        "FREQ=YEARLY"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_recurring_character_birthdays() {
        let calendar = build_ics(
            "sr",
            "崩坏：星穹铁道",
            &[CharacterBirthdayResponse {
                character_id: "1001".to_owned(),
                character_name: "三月七".to_owned(),
                character_icon: Some("https://example.com/march.webp".to_owned()),
                birthday_month: 3,
                birthday_day: 7,
            }],
            &BirthdayReminderOptions::default(),
        );
        assert!(calendar.contains("DTSTART;VALUE=DATE:20000307\r\n"));
        assert!(calendar.contains("RRULE:FREQ=YEARLY\r\n"));
        assert!(calendar.contains("SUMMARY:三月七生日\r\n"));
        assert!(calendar.contains("NAME;LANGUAGE=zh-CN:崩坏：星穹铁道角色生日\r\n"));
        assert!(calendar.contains("X-WR-CALNAME:崩坏：星穹铁道角色生日\r\n"));
        assert!(calendar.ends_with("END:VCALENDAR\r\n"));
    }

    #[test]
    fn exports_leap_day_birthdays_on_the_last_day_of_february() {
        let calendar = build_ics(
            "ys",
            "原神",
            &[CharacterBirthdayResponse {
                character_id: "10000032".to_owned(),
                character_name: "班尼特".to_owned(),
                character_icon: Some("https://example.com/bennett.webp".to_owned()),
                birthday_month: 2,
                birthday_day: 29,
            }],
            &BirthdayReminderOptions::default(),
        );

        assert!(calendar.contains("DTSTART;VALUE=DATE:20000229\r\n"));
        assert!(calendar.contains("RRULE:FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=-1\r\n"));
        assert!(calendar.contains("NAME;LANGUAGE=zh-CN:原神角色生日\r\n"));
        assert!(calendar.contains("X-WR-CALNAME:原神角色生日\r\n"));
    }

    #[test]
    fn parses_birthday_filters_and_reminder_options_from_one_query() {
        let uri = "/calendar/character-birthdays.ics?birthday_month=8&gender=female&reminder_time=09%3A30&reminder_minutes_before=1440"
            .parse()
            .expect("test URI should be valid");
        let Query(query) = Query::<CharacterBirthdayIcsQuery>::try_from_uri(&uri)
            .expect("birthday ICS query should deserialize");
        let (filter, options) = query.into_parts();

        assert_eq!(filter.birthday_month, Some(8));
        assert_eq!(filter.gender.as_deref(), Some("female"));
        assert_eq!(options.reminder_time.as_deref(), Some("09:30"));
        assert_eq!(options.reminder_minutes_before, Some(1440));
    }

    #[test]
    fn exports_same_day_and_advance_birthday_reminders() {
        let options = CharacterBirthdayIcsOptions {
            reminder_time: Some("09:30".to_owned()),
            reminder_minutes_before: Some(1440),
        }
        .validate()
        .expect("test reminder options should be valid");
        let calendar = build_ics(
            "ys",
            "原神",
            &[CharacterBirthdayResponse {
                character_id: "10000032".to_owned(),
                character_name: "班尼特".to_owned(),
                character_icon: None,
                birthday_month: 2,
                birthday_day: 29,
            }],
            &options,
        );

        assert!(calendar.contains("TRIGGER;RELATED=START:-P1D\r\n"));
        assert!(calendar.contains("TRIGGER;RELATED=START:PT9H30M\r\n"));
        assert!(calendar.contains("DESCRIPTION:班尼特的生日即将到来\r\n"));
        assert!(calendar.contains("DESCRIPTION:今天是班尼特的生日\r\n"));
        assert_eq!(calendar.matches("BEGIN:VALARM").count(), 2);
    }

    #[test]
    fn rejects_invalid_birthday_reminder_options() {
        for reminder_time in ["9:00", "24:00", "09:30:00"] {
            let options = CharacterBirthdayIcsOptions {
                reminder_time: Some(reminder_time.to_owned()),
                reminder_minutes_before: None,
            };
            assert!(matches!(options.validate(), Err(AppError::BadRequest(_))));
        }

        let options = CharacterBirthdayIcsOptions {
            reminder_time: None,
            reminder_minutes_before: Some(MAX_BIRTHDAY_REMINDER_MINUTES + 1),
        };
        assert!(matches!(options.validate(), Err(AppError::BadRequest(_))));
    }

    #[test]
    fn rejects_unsupported_calendar_games_as_not_found() {
        assert!(matches!(
            validate_calendar_game("planet"),
            Err(AppError::NotFound(message)) if message == "character calendar is not available for game planet"
        ));
        assert!(validate_calendar_game("ys").is_ok());
    }

    #[test]
    fn exposes_owned_character_icons_as_absolute_urls() {
        let item = birthday(
            "10000032".to_owned(),
            "班尼特".to_owned(),
            "/assets/game-data/ys/bennett.webp".to_owned(),
            Some(2),
            Some(29),
            "https://api.example.com",
        )
        .expect("birthday should be present");

        assert_eq!(
            item.character_icon.as_deref(),
            Some("https://api.example.com/assets/game-data/ys/bennett.webp")
        );
    }
}
