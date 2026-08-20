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
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    http::{
        error::AppError,
        path::GamePath,
        response::{ErrorResponse, ListResponse, public_asset_url},
    },
    state::AppState,
};

const CHARACTER_LIMIT: u64 = 1_000;

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
    description = "按游戏导出每年重复的角色生日事件，可按名称和月份筛选",
    params(GamePath, CharacterBirthdayQuery),
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
    Query(query): Query<CharacterBirthdayQuery>,
) -> Result<Response, AppError> {
    let items = list_birthdays(&state, &game_id, query).await?;
    let game = state
        .application()
        .find_game(&game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("game {game_id} not found")))?;
    let calendar = build_ics(&game_id, &game.name_zh, &items);
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
            return Err(AppError::BadRequest(
                "character calendar only supports ys, sr and zzz".to_owned(),
            ));
        }
    };
    Ok(items)
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

fn build_ics(game_id: &str, game_name: &str, items: &[CharacterBirthdayResponse]) -> String {
    let calendar_name = format!("{game_name}角色生日");
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_owned(),
        "VERSION:2.0".to_owned(),
        "PRODID:-//Akasha//Character Birthdays//ZH-CN".to_owned(),
        "CALSCALE:GREGORIAN".to_owned(),
        "METHOD:PUBLISH".to_owned(),
        format!("NAME;LANGUAGE=zh-CN:{}", ics_escape(&calendar_name)),
        format!("X-WR-CALNAME:{}", ics_escape(&calendar_name)),
    ];
    for item in items {
        lines.extend([
            "BEGIN:VEVENT".to_owned(),
            format!(
                "UID:character-{}-{}@akasha",
                game_id,
                ics_escape(&item.character_id)
            ),
            "DTSTAMP:19700101T000000Z".to_owned(),
            format!(
                "DTSTART;VALUE=DATE:2000{:02}{:02}",
                item.birthday_month, item.birthday_day
            ),
            birthday_recurrence(item.birthday_month, item.birthday_day).to_owned(),
            format!("SUMMARY:{}生日", ics_escape(&item.character_name)),
            "TRANSP:TRANSPARENT".to_owned(),
            "END:VEVENT".to_owned(),
        ]);
    }
    lines.push("END:VCALENDAR".to_owned());
    format!("{}\r\n", lines.join("\r\n"))
}

/// 让 2 月 29 日生日在非闰年回退到二月最后一天
fn birthday_recurrence(month: i16, day: i16) -> &'static str {
    if month == 2 && day == 29 {
        "RRULE:FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=-1"
    } else {
        "RRULE:FREQ=YEARLY"
    }
}

fn ics_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(',', "\\,")
        .replace(';', "\\;")
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
        );

        assert!(calendar.contains("DTSTART;VALUE=DATE:20000229\r\n"));
        assert!(calendar.contains("RRULE:FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=-1\r\n"));
        assert!(calendar.contains("NAME;LANGUAGE=zh-CN:原神角色生日\r\n"));
        assert!(calendar.contains("X-WR-CALNAME:原神角色生日\r\n"));
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
