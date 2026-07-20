use chrono::{DateTime, Days, FixedOffset, NaiveDate, TimeZone};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{http::error::AppError, http::path::GamePath};

use akasha_db::repositories::news::ListNewsFilter;

const DEFAULT_PAGE_LIMIT: u64 = 20;
const MAX_PAGE_LIMIT: u64 = 100;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub(super) struct NewsDetailPath {
    /// 新闻 ID
    pub news_id: String,
    /// 游戏 ID
    pub game_id: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsListQuery {
    /// 来源 ID
    pub source_id: String,
    /// 标题关键词，空格分隔，- 前缀表示排除词
    pub q: Option<String>,
    /// 标签，逗号分隔
    pub tags: Option<String>,
    /// 筛选新闻类型
    pub news_type: Option<String>,
    /// 时间范围 YYYYMMDD-YYYYMMDD
    pub during: Option<String>,
    /// 获取数量，默认 20
    pub limit: Option<u64>,
    /// 偏移，默认 0
    pub offset: Option<u64>,
    /// 反转
    pub reverse: Option<bool>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsDetailQuery {
    /// 来源 ID
    pub source_id: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsTagsQuery {
    /// 来源 ID
    pub source_id: String,
}

impl NewsListQuery {
    pub(super) fn into_filter(
        self,
        GamePath { game_id }: GamePath,
    ) -> Result<ListNewsFilter, AppError> {
        let (start_publish_time, end_publish_time) = match self.during.as_deref() {
            Some(during) => parse_time_interval(during).map_err(AppError::BadRequest)?,
            None => (None, None),
        };

        Ok(ListNewsFilter {
            source_id: self.source_id,
            game_id,
            q: self.q,
            tags: parse_csv(self.tags),
            news_type: self.news_type,
            start_publish_time,
            end_publish_time,
            limit: self
                .limit
                .unwrap_or(DEFAULT_PAGE_LIMIT)
                .clamp(1, MAX_PAGE_LIMIT),
            offset: self.offset.unwrap_or(0),
            reverse: self.reverse.unwrap_or(false),
        })
    }
}

fn parse_csv(value: Option<String>) -> Option<Vec<String>> {
    let items = value?
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    (!items.is_empty()).then_some(items)
}

fn parse_time_interval(
    during: &str,
) -> Result<(Option<DateTime<FixedOffset>>, Option<DateTime<FixedOffset>>), String> {
    fn parse_date(value: &str) -> Result<NaiveDate, String> {
        if value.len() != 8 || !value.chars().all(|ch| ch.is_ascii_digit()) {
            return Err("日期格式应为 YYYYMMDD".to_string());
        }

        NaiveDate::parse_from_str(value, "%Y%m%d").map_err(|_| "日期无效".to_string())
    }

    fn at_day_start(
        date: NaiveDate,
        timezone: FixedOffset,
    ) -> Result<DateTime<FixedOffset>, String> {
        timezone
            .from_local_datetime(
                &date
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| "日期无效".to_string())?,
            )
            .single()
            .ok_or_else(|| "日期无效".to_string())
    }

    let during = during.trim();
    let Some((start, end)) = during.split_once('-') else {
        return Err("时间范围格式应为 YYYYMMDD-YYYYMMDD、YYYYMMDD- 或 -YYYYMMDD".to_string());
    };

    if end.contains('-') {
        return Err("时间范围格式只能包含一个 '-'".to_string());
    }

    if start.is_empty() && end.is_empty() {
        return Err("时间范围至少需要开始日期或结束日期".to_string());
    }

    let timezone = FixedOffset::east_opt(8 * 60 * 60).expect("valid fixed timezone");
    let start = (!start.is_empty())
        .then(|| parse_date(start).and_then(|date| at_day_start(date, timezone)))
        .transpose()?;
    let end = (!end.is_empty())
        .then(|| {
            parse_date(end).and_then(|date| {
                date.checked_add_days(Days::new(1))
                    .ok_or_else(|| "结束日期超出有效范围".to_string())
                    .and_then(|date| at_day_start(date, timezone))
            })
        })
        .transpose()?;

    if let (Some(start), Some(end)) = (start, end)
        && start >= end
    {
        return Err("开始日期必须早于或等于结束日期".to_string());
    }

    Ok((start, end))
}
