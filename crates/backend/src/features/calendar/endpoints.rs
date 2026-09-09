use akasha_application::characters::{
    SrCharacterListFilter, YsCharacterListFilter, ZzzCharacterListFilter,
};
use chrono::NaiveDate;

use super::ics::{AlarmOffset, AlarmRelation, IcsCalendar, IcsEvent};
use crate::{
    http::{error::AppError, response::public_asset_url},
    state::AppState,
};

const CHARACTER_LIMIT: u64 = 1_000;

pub(super) struct BirthdayReminderOptions {
    pub(super) reminder_time_minutes: Option<u32>,
    pub(super) reminder_minutes_before: Option<u32>,
}

#[derive(Debug, Clone)]
pub(super) struct CharacterBirthdayResponse {
    pub(super) character_id: String,
    pub(super) character_name: String,
    pub(super) character_icon: Option<String>,
    pub(super) birthday_month: i16,
    pub(super) birthday_day: i16,
}

/// 从角色数据读取可投影到日程的生日
pub(super) async fn list_birthdays(
    state: &AppState,
    game_id: &str,
) -> Result<Vec<CharacterBirthdayResponse>, AppError> {
    let asset_base_url = &state.config().asset_base_url;
    let items = match game_id {
        "ys" => state
            .application()
            .list_ys_characters(YsCharacterListFilter {
                query: None,
                element: None,
                weapon_type: None,
                rarity: None,
                region: None,
                affiliation: None,
                voice_actor: None,
                birthday_month: None,
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
                query: None,
                path: None,
                combat_type: None,
                rarity: None,
                camp: None,
                voice_actor: None,
                birthday_month: None,
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
                query: None,
                specialty_id: None,
                specialty: None,
                element_id: None,
                element: None,
                hit_type_id: None,
                hit_type: None,
                camp_id: None,
                camp: None,
                rarity: None,
                gender: None,
                special_element: None,
                birthday_month: None,
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

fn birthday(
    character_id: String,
    character_name: String,
    icon: String,
    month: Option<i16>,
    day: Option<i16>,
    asset_base_url: &str,
) -> Option<CharacterBirthdayResponse> {
    Some(CharacterBirthdayResponse {
        character_id,
        character_name,
        character_icon: icon
            .starts_with("/assets/game-data/")
            .then_some(icon)
            .and_then(|value| public_asset_url(asset_base_url, Some(value))),
        birthday_month: month?,
        birthday_day: day?,
    })
}

pub(super) fn append_birthdays_to_ics(
    calendar: &mut IcsCalendar,
    game_id: &str,
    items: &[CharacterBirthdayResponse],
    options: &BirthdayReminderOptions,
) {
    for item in items {
        let date =
            NaiveDate::from_ymd_opt(2000, item.birthday_month as u32, item.birthday_day as u32)
                .expect("stored birthday should be a valid date in leap year 2000");
        let mut event = IcsEvent::new(&format!("character-{game_id}-{}@akasha", item.character_id))
            .starts_on(date)
            .recurrence(birthday_recurrence(item.birthday_month, item.birthday_day))
            .summary(&format!("{}生日", item.character_name))
            .transparent();
        if let Some(minutes) = options.reminder_minutes_before {
            event = event.display_alarm(
                AlarmRelation::Start,
                AlarmOffset::Before(minutes),
                &format!("{}的生日即将到来", item.character_name),
            );
        }
        if let Some(minutes) = options.reminder_time_minutes
            && !(minutes == 0 && options.reminder_minutes_before == Some(0))
        {
            event = event.display_alarm(
                AlarmRelation::Start,
                AlarmOffset::After(minutes),
                &format!("今天是{}的生日", item.character_name),
            );
        }
        calendar.push_event(event);
    }
}

fn birthday_recurrence(month: i16, day: i16) -> &'static str {
    if month == 2 && day == 29 {
        // ICS 为兼容不支持负数 BYMONTHDAY 的日历客户端，平年落在 3 月 1 日
        "FREQ=YEARLY;BYYEARDAY=60"
    } else {
        "FREQ=YEARLY"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_leap_day_birthdays_on_the_sixtieth_day() {
        let mut calendar = IcsCalendar::new("-//Akasha//Test//ZH-CN", "原神日程");
        append_birthdays_to_ics(
            &mut calendar,
            "ys",
            &[CharacterBirthdayResponse {
                character_id: "10000032".to_owned(),
                character_name: "班尼特".to_owned(),
                character_icon: None,
                birthday_month: 2,
                birthday_day: 29,
            }],
            &BirthdayReminderOptions {
                reminder_time_minutes: None,
                reminder_minutes_before: None,
            },
        );
        let output = calendar.finish();
        assert!(output.contains("DTSTART;VALUE=DATE:20000229\r\n"));
        assert!(output.contains("RRULE:FREQ=YEARLY;BYYEARDAY=60\r\n"));
    }
}
