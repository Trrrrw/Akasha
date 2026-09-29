use chrono::{DateTime, Datelike, FixedOffset, Months, NaiveDate, TimeZone};

use super::{CalendarEvent, ListCalendarEventsFilter};

const GAME_ACTIVITY: &str = "游戏内活动";

struct MonthlyActivity {
    id: &'static str,
    title: &'static str,
    day: u32,
    first_refresh: (i32, u32, u32, u32),
    cover: Option<&'static str>,
    url: &'static str,
}

const ACTIVITIES: &[MonthlyActivity] = &[
    MonthlyActivity {
        id: "imaginarium-theater",
        title: "幻想真境剧诗",
        day: 1,
        // 首期在 10:00 开启，之后按每月首日 04:00 刷新
        first_refresh: (2024, 7, 1, 10),
        cover: Some("/assets/games/ys/imaginarium-theater.png"),
        url: "https://www.miyoushe.com/ys/article/53148680",
    },
    MonthlyActivity {
        id: "spiral-abyss",
        title: "深境螺旋",
        day: 16,
        // 从该期起改为每月 16 日刷新
        first_refresh: (2024, 6, 16, 4),
        cover: None,
        url: "https://ys.mihoyo.com/",
    },
];

pub(super) fn enabled(filter: &ListCalendarEventsFilter) -> bool {
    filter.game_id == "ys"
        && filter.limit > 0
        && (filter.kinds.is_empty() || filter.kinds.iter().any(|kind| kind == GAME_ACTIVITY))
}

/// 按北京时间生成与查询范围相交的完整玩法周期，结束时间不包含在本期内
pub(super) fn materialize(
    filter: &ListCalendarEventsFilter,
    game_cover: Option<String>,
) -> Vec<CalendarEvent> {
    if !enabled(filter) {
        return Vec::new();
    }
    let timezone = FixedOffset::east_opt(8 * 60 * 60).expect("UTC+8 should be valid");
    let from = filter.start_time.with_timezone(&timezone).date_naive();
    let first_month =
        NaiveDate::from_ymd_opt(from.year(), from.month(), 1).expect("month start should be valid");
    // 查询从刷新前或月中开始时，也需返回仍在进行的上一期
    let mut month = first_month
        .checked_sub_months(Months::new(1))
        .unwrap_or(first_month);
    let mut events = Vec::new();
    while let Some(next_month) = month.checked_add_months(Months::new(1)) {
        for activity in ACTIVITIES {
            let start = refresh(timezone, month, activity);
            let end = refresh(timezone, next_month, activity);
            let (year, first_month, day, hour) = activity.first_refresh;
            let first_refresh = timezone
                .with_ymd_and_hms(year, first_month, day, hour, 0, 0)
                .single()
                .expect("first refresh should be valid");
            if start < first_refresh || start >= filter.end_time || end <= filter.start_time {
                continue;
            }
            let id = format!("monthly-{}-{}", activity.id, month.format("%Y-%m"));
            events.push(CalendarEvent {
                game_id: "ys".to_owned(),
                id: id.clone(),
                kind: GAME_ACTIVITY.to_owned(),
                title: format!(
                    "{} · {}年{}月期",
                    activity.title,
                    month.year(),
                    month.month()
                ),
                start_time: start,
                end_time: end,
                version_id: None,
                start_version_id: None,
                cover: activity
                    .cover
                    .map(str::to_owned)
                    .or_else(|| game_cover.clone()),
                labels: vec![activity.title.to_owned()],
                source_id: "fixed-schedule".to_owned(),
                source_news_id: id,
                source_url: activity.url.to_owned(),
                source_hash: "monthly-utc8-04:00".to_owned(),
            });
        }
        let next_start = timezone
            .from_local_datetime(
                &next_month
                    .and_hms_opt(0, 0, 0)
                    .expect("midnight should be valid"),
            )
            .single()
            .expect("fixed offset should resolve local time");
        if next_start >= filter.end_time {
            break;
        }
        month = next_month;
    }
    events
}

fn refresh(
    timezone: FixedOffset,
    month: NaiveDate,
    activity: &MonthlyActivity,
) -> DateTime<FixedOffset> {
    let (year, first_month, _, first_hour) = activity.first_refresh;
    let hour = if (month.year(), month.month()) == (year, first_month) {
        first_hour
    } else {
        4
    };
    timezone
        .with_ymd_and_hms(month.year(), month.month(), activity.day, hour, 0, 0)
        .single()
        .expect("monthly refresh should be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(start: &str, end: &str) -> ListCalendarEventsFilter {
        ListCalendarEventsFilter {
            game_id: "ys".to_owned(),
            start_time: DateTime::parse_from_rfc3339(start).unwrap(),
            end_time: DateTime::parse_from_rfc3339(end).unwrap(),
            kinds: vec![GAME_ACTIVITY.to_owned()],
            limit: 500,
        }
    }

    #[test]
    fn includes_ongoing_periods_and_handles_year_rollover() {
        let events = materialize(
            &filter("2026-12-20T00:00:00+08:00", "2027-02-01T00:00:00+08:00"),
            Some("/default.avif".to_owned()),
        );
        assert_eq!(events.len(), 4);
        let theater = events
            .iter()
            .find(|event| event.id == "monthly-imaginarium-theater-2026-12")
            .unwrap();
        assert_eq!(theater.start_time.to_rfc3339(), "2026-12-01T04:00:00+08:00");
        assert_eq!(theater.end_time.to_rfc3339(), "2027-01-01T04:00:00+08:00");
        assert_eq!(theater.title, "幻想真境剧诗 · 2026年12月期");
        assert_eq!(
            events
                .iter()
                .find(|event| event.id == "monthly-imaginarium-theater-2027-01")
                .unwrap()
                .title,
            "幻想真境剧诗 · 2027年1月期"
        );
        assert_eq!(theater.cover.as_deref(), ACTIVITIES[0].cover);
        let abyss = events
            .iter()
            .find(|event| event.id == "monthly-spiral-abyss-2026-12")
            .unwrap();
        assert_eq!(abyss.start_time.to_rfc3339(), "2026-12-16T04:00:00+08:00");
        assert_eq!(abyss.end_time.to_rfc3339(), "2027-01-16T04:00:00+08:00");
        assert_eq!(abyss.title, "深境螺旋 · 2026年12月期");
        assert_eq!(abyss.cover.as_deref(), Some("/default.avif"));
    }

    #[test]
    fn switches_periods_at_exact_refresh_time() {
        let events = materialize(
            &filter("2028-02-01T04:00:00+08:00", "2028-03-01T04:00:00+08:00"),
            None,
        );
        let theater = events
            .iter()
            .filter(|event| event.labels.iter().any(|label| label == "幻想真境剧诗"))
            .collect::<Vec<_>>();
        assert_eq!(theater.len(), 1);
        assert_eq!(theater[0].id, "monthly-imaginarium-theater-2028-02");
        assert_eq!((theater[0].end_time - theater[0].start_time).num_days(), 29);
        assert_eq!(
            events
                .iter()
                .filter(|event| event.labels.iter().any(|label| label == "深境螺旋"))
                .count(),
            2
        );
    }

    #[test]
    fn does_not_create_periods_before_launch_or_for_other_games_and_kinds() {
        let mut query = filter("2024-06-01T00:00:00+08:00", "2024-07-01T10:00:00+08:00");
        let events = materialize(&query, None);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "monthly-spiral-abyss-2024-06");
        query.game_id = "sr".to_owned();
        assert!(materialize(&query, None).is_empty());
        query.game_id = "ys".to_owned();
        query.kinds = vec!["卡池".to_owned()];
        assert!(materialize(&query, None).is_empty());
    }
}
