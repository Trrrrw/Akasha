use super::*;

#[test]
fn parses_repeated_ics_selectors() {
    let uri = "/calendar.ics?include=版本日程&include=角色生日&exclude=版本日程%3A版本维护"
        .parse()
        .expect("test URI should be valid");
    let MultiQuery(query) =
        MultiQuery::<CalendarIcsQuery>::try_from_uri(&uri).expect("ICS query should deserialize");
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
    assert!(values.contains(&"游戏内活动:幻想真境剧诗"));
    assert!(values.contains(&"游戏内活动:深境螺旋"));
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
    assert_eq!(filter.event_kinds(), [GAME_ACTIVITY]);
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

#[tokio::test]
async fn fixed_activities_use_shared_filters_assets_limits_and_ics_options() {
    use akasha_application::ApplicationServices;
    use akasha_db::{Db, DbOptions};

    let application = ApplicationServices::new(
        Db::init(DbOptions {
            sqlite_path: ":memory:".to_owned(),
        })
        .await
        .expect("isolated calendar database should initialize"),
    );
    let query = ListCalendarEventsFilter {
        game_id: "ys".to_owned(),
        start_time: chrono::DateTime::parse_from_rfc3339("2026-10-01T04:00:00+08:00").unwrap(),
        end_time: chrono::DateTime::parse_from_rfc3339("2026-11-01T04:00:00+08:00").unwrap(),
        kinds: vec![GAME_ACTIVITY.to_owned()],
        limit: 500,
    };
    let events = application
        .list_calendar_events(query.clone())
        .await
        .unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].id, "monthly-spiral-abyss-2026-09");
    assert_eq!(
        events[0].cover.as_deref(),
        Some("/assets/games/ys/cover.avif")
    );
    let limited = application
        .list_calendar_events(ListCalendarEventsFilter { limit: 1, ..query })
        .await
        .unwrap();
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].id, events[0].id);

    let capability = capabilities("ys").unwrap();
    let include =
        SelectorFilter::new(&["游戏内活动:幻想真境剧诗".to_owned()], &[], &capability).unwrap();
    let exclude = SelectorFilter::new(
        &[GAME_ACTIVITY.to_owned()],
        &["游戏内活动:深境螺旋".to_owned()],
        &capability,
    )
    .unwrap();
    let items = events
        .into_iter()
        .filter(|event| include.matches(&event.kind, &event.labels))
        .map(|event| {
            assert!(exclude.matches(&event.kind, &event.labels));
            CalendarEntryResponse::from_event(event, "https://assets.example")
        })
        .collect::<Vec<_>>();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "幻想真境剧诗 · 2026年10月期");
    assert_eq!(items[0].start, "2026-09-30T20:00:00Z");
    assert_eq!(items[0].end, "2026-10-31T20:00:00Z");
    assert_eq!(
        items[0].cover.as_deref(),
        Some("https://assets.example/assets/games/ys/imaginarium-theater.png")
    );
    assert!(!items[0].all_day);

    for (mode, event_count) in [(CalendarIcsMode::Span, 1), (CalendarIcsMode::Milestones, 2)] {
        let mut calendar = IcsCalendar::new("-//Akasha//Test//ZH-CN", "原神日程");
        append_timed_to_ics(
            &mut calendar,
            "ys",
            &items,
            &CalendarIcsOptions {
                event_mode: mode,
                start_reminder_minutes: Some(30),
                end_reminder_minutes: Some(1440),
                ..Default::default()
            },
        );
        let output = calendar.finish();
        assert_eq!(output.matches("BEGIN:VEVENT").count(), event_count);
        assert!(output.contains("DTSTART:20260930T200000Z"));
        assert!(output.contains("幻想真境剧诗 · 2026年10月期"));
        assert!(output.contains("20261031T200000Z"));
        assert!(output.contains("TRIGGER;RELATED=START:-PT30M"));
        assert!(output.contains("-P1D"));
    }
}
