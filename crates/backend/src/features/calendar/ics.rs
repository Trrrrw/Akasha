use chrono::{DateTime, NaiveDate, Utc};

const DTSTAMP: &str = "19700101T000000Z";

/// 闹钟相对于事件的时间基准
#[derive(Debug, Clone, Copy)]
pub(super) enum AlarmRelation {
    Start,
    End,
}

/// 闹钟相对于时间基准的偏移方向和分钟数
#[derive(Debug, Clone, Copy)]
pub(super) enum AlarmOffset {
    Before(u32),
    After(u32),
}

/// 一个可序列化的 iCalendar 日历
pub(super) struct IcsCalendar {
    properties: Vec<String>,
    events: Vec<IcsEvent>,
}

impl IcsCalendar {
    /// 使用项目标识和显示名称创建日历
    pub(super) fn new(product_id: &str, name: &str) -> Self {
        Self {
            properties: vec![
                "VERSION:2.0".to_owned(),
                format!("PRODID:{product_id}"),
                "CALSCALE:GREGORIAN".to_owned(),
                "METHOD:PUBLISH".to_owned(),
                text_property("NAME;LANGUAGE=zh-CN", name),
                text_property("X-WR-CALNAME", name),
            ],
            events: Vec::new(),
        }
    }

    /// 向日历添加一个事件
    pub(super) fn push_event(&mut self, event: IcsEvent) {
        self.events.push(event);
    }

    /// 生成使用 CRLF 和 UTF-8 安全折行的 ICS 文本
    pub(super) fn finish(self) -> String {
        let mut lines = Vec::new();
        lines.push("BEGIN:VCALENDAR".to_owned());
        lines.extend(self.properties);
        for event in self.events {
            event.append_to(&mut lines);
        }
        lines.push("END:VCALENDAR".to_owned());
        render_lines(&lines)
    }
}

/// 一个 iCalendar 事件
pub(super) struct IcsEvent {
    properties: Vec<String>,
    alarms: Vec<IcsAlarm>,
}

impl IcsEvent {
    /// 使用稳定 UID 创建事件
    pub(super) fn new(uid: &str) -> Self {
        Self {
            properties: vec![text_property("UID", uid), format!("DTSTAMP:{DTSTAMP}")],
            alarms: Vec::new(),
        }
    }

    /// 设置 UTC 日期时间起点
    pub(super) fn starts_at(mut self, start: DateTime<Utc>) -> Self {
        self.properties
            .push(format!("DTSTART:{}", format_utc(start)));
        self
    }

    /// 设置全天日期起点
    pub(super) fn starts_on(mut self, start: NaiveDate) -> Self {
        self.properties
            .push(format!("DTSTART;VALUE=DATE:{}", start.format("%Y%m%d")));
        self
    }

    /// 设置 UTC 日期时间终点
    pub(super) fn ends_at(mut self, end: DateTime<Utc>) -> Self {
        self.properties.push(format!("DTEND:{}", format_utc(end)));
        self
    }

    /// 设置事件标题
    pub(super) fn summary(mut self, summary: &str) -> Self {
        self.properties.push(text_property("SUMMARY", summary));
        self
    }

    /// 设置事件关联地址
    pub(super) fn url(mut self, url: &str) -> Self {
        self.properties
            .push(format!("URL:{}", single_line_value(url)));
        self
    }

    /// 设置原始重复规则
    pub(super) fn recurrence(mut self, rule: &str) -> Self {
        self.properties.push(format!("RRULE:{rule}"));
        self
    }

    /// 将事件标记为不占用忙碌时间
    pub(super) fn transparent(mut self) -> Self {
        self.properties.push("TRANSP:TRANSPARENT".to_owned());
        self
    }

    /// 添加一个相对于开始或结束触发的显示提醒
    pub(super) fn display_alarm(
        mut self,
        relation: AlarmRelation,
        offset: AlarmOffset,
        description: &str,
    ) -> Self {
        self.alarms.push(IcsAlarm {
            relation,
            offset,
            description: description.to_owned(),
        });
        self
    }

    fn append_to(self, lines: &mut Vec<String>) {
        lines.push("BEGIN:VEVENT".to_owned());
        lines.extend(self.properties);
        for alarm in self.alarms {
            alarm.append_to(lines);
        }
        lines.push("END:VEVENT".to_owned());
    }
}

struct IcsAlarm {
    relation: AlarmRelation,
    offset: AlarmOffset,
    description: String,
}

impl IcsAlarm {
    fn append_to(self, lines: &mut Vec<String>) {
        let relation = match self.relation {
            AlarmRelation::Start => "START",
            AlarmRelation::End => "END",
        };
        lines.extend([
            "BEGIN:VALARM".to_owned(),
            "ACTION:DISPLAY".to_owned(),
            format!("TRIGGER;RELATED={relation}:{}", duration(self.offset)),
            text_property("DESCRIPTION", &self.description),
            "END:VALARM".to_owned(),
        ]);
    }
}

fn format_utc(value: DateTime<Utc>) -> String {
    value.format("%Y%m%dT%H%M%SZ").to_string()
}

fn duration(offset: AlarmOffset) -> String {
    let (negative, minutes) = match offset {
        AlarmOffset::Before(minutes) => (minutes > 0, minutes),
        AlarmOffset::After(minutes) => (false, minutes),
    };
    if minutes == 0 {
        return "PT0M".to_owned();
    }

    let days = minutes / (24 * 60);
    let remainder = minutes % (24 * 60);
    let hours = remainder / 60;
    let minutes = remainder % 60;
    let mut duration = if negative { "-P" } else { "P" }.to_owned();
    if days > 0 {
        duration.push_str(&format!("{days}D"));
    }
    if hours > 0 || minutes > 0 || days == 0 {
        duration.push('T');
        if hours > 0 {
            duration.push_str(&format!("{hours}H"));
        }
        if minutes > 0 {
            duration.push_str(&format!("{minutes}M"));
        }
    }
    duration
}

fn text_property(name: &str, value: &str) -> String {
    format!("{name}:{}", escape_text(value))
}

fn single_line_value(value: &str) -> String {
    value.replace(['\r', '\n'], "")
}

fn escape_text(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(',', "\\,")
        .replace(';', "\\;")
}

fn render_lines(lines: &[String]) -> String {
    let mut output = String::new();
    for line in lines {
        append_folded_line(&mut output, line);
    }
    output
}

fn append_folded_line(output: &mut String, line: &str) {
    let mut remaining = line;
    let mut first = true;
    loop {
        let byte_limit = if first { 75 } else { 74 };
        let mut end = remaining.len().min(byte_limit);
        while !remaining.is_char_boundary(end) {
            end -= 1;
        }
        if !first {
            output.push(' ');
        }
        output.push_str(&remaining[..end]);
        output.push_str("\r\n");
        if end == remaining.len() {
            break;
        }
        remaining = &remaining[end..];
        first = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_display_alarms_relative_to_start_and_end() {
        let event = IcsEvent::new("event-1@akasha")
            .starts_at(
                DateTime::parse_from_rfc3339("2026-08-19T10:00:00+08:00")
                    .expect("test time should be valid")
                    .with_timezone(&Utc),
            )
            .ends_at(
                DateTime::parse_from_rfc3339("2026-09-22T03:59:00+08:00")
                    .expect("test time should be valid")
                    .with_timezone(&Utc),
            )
            .display_alarm(
                AlarmRelation::Start,
                AlarmOffset::Before(60),
                "活动即将开始",
            )
            .display_alarm(
                AlarmRelation::End,
                AlarmOffset::Before(24 * 60 + 30),
                "活动即将结束",
            )
            .display_alarm(
                AlarmRelation::Start,
                AlarmOffset::After(9 * 60),
                "当天上午提醒",
            );
        let mut calendar = IcsCalendar::new("-//Akasha//Test//ZH-CN", "测试日历");
        calendar.push_event(event);
        let output = calendar.finish();

        assert!(output.contains("TRIGGER;RELATED=START:-PT1H\r\n"));
        assert!(output.contains("TRIGGER;RELATED=END:-P1DT30M\r\n"));
        assert!(output.contains("TRIGGER;RELATED=START:PT9H\r\n"));
        assert_eq!(output.matches("BEGIN:VALARM").count(), 3);
    }

    #[test]
    fn folds_long_utf8_content_without_splitting_characters() {
        let mut calendar = IcsCalendar::new("-//Akasha//Test//ZH-CN", "测试日历");
        calendar.push_event(
            IcsEvent::new("event-1@akasha")
                .starts_on(NaiveDate::from_ymd_opt(2026, 8, 19).expect("test date should be valid"))
                .summary(&"很长的中文活动标题".repeat(10)),
        );
        let output = calendar.finish();

        assert!(output.lines().all(|line| line.len() <= 75));
        assert!(output.lines().any(|line| line.starts_with(' ')));
    }

    #[test]
    fn escapes_text_properties() {
        let mut calendar = IcsCalendar::new("-//Akasha//Test//ZH-CN", "测试,日历");
        calendar.push_event(
            IcsEvent::new("event-1@akasha")
                .starts_on(NaiveDate::from_ymd_opt(2026, 8, 19).expect("test date should be valid"))
                .summary("第一行;内容\n第二行"),
        );
        let output = calendar.finish();

        assert!(output.contains("NAME;LANGUAGE=zh-CN:测试\\,日历\r\n"));
        assert!(output.contains("SUMMARY:第一行\\;内容\\n第二行\r\n"));
    }
}
