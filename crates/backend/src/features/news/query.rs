use chrono::{DateTime, Days, FixedOffset, NaiveDate, TimeZone};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{http::error::AppError, http::path::GamePath};

use akasha_application::news::ListNewsFilter;

const DEFAULT_PAGE_LIMIT: u64 = 20;
const MAX_PAGE_LIMIT: u64 = 100;

/// 新闻发布日期的右开时间范围
type PublishTimeRange = (Option<DateTime<FixedOffset>>, Option<DateTime<FixedOffset>>);

/// 新闻详情路径中的游戏和新闻标识
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub(super) struct NewsDetailPath {
    /// 新闻 ID
    pub news_id: String,
    /// 游戏 ID
    pub game_id: String,
}

/// 新闻标签剧集路径中的游戏和标签标识
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub(super) struct NewsSeriesPath {
    /// 游戏 ID
    pub game_id: String,
    /// 作为剧集名称的新闻标签
    pub tag_name: String,
}

/// 新闻标签单集路径中的游戏、标签和新闻标识
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub(super) struct NewsSeriesEpisodePath {
    /// 游戏 ID
    pub game_id: String,
    /// 作为剧集名称的新闻标签
    pub tag_name: String,
    /// 视频新闻 ID
    pub news_id: String,
}

/// 新闻列表接口接受的查询参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsListQuery {
    /// 新闻来源 ID，例如 web_cn 或 mys
    pub source_id: String,
    /// 标题关键词，空格分隔，使用 - 前缀排除关键词
    pub q: Option<String>,
    /// 任一匹配标签，逗号分隔，__untagged__ 表示未分类
    pub tags: Option<String>,
    /// 新闻类型，仅支持 article 或 video
    pub news_type: Option<String>,
    /// 发布日期范围，格式为 YYYYMMDD-YYYYMMDD，结束日期包含完整当天
    pub during: Option<String>,
    /// 每页数量，默认 20，最大 100
    pub limit: Option<u64>,
    /// 分页偏移，默认 0
    pub offset: Option<u64>,
    /// 是否按发布时间升序排列，默认 false 表示降序
    pub reverse: Option<bool>,
}

/// RSS 接口接受的新闻筛选参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsRssQuery {
    /// 新闻来源 ID，例如 web_cn 或 mys
    pub source_id: String,
    /// 标题关键词，空格分隔，使用 - 前缀排除关键词
    pub q: Option<String>,
    /// 任一匹配标签，逗号分隔，__untagged__ 表示未分类
    pub tags: Option<String>,
    /// 新闻类型，仅支持 article 或 video
    pub news_type: Option<String>,
    /// 发布日期范围，格式为 YYYYMMDD-YYYYMMDD，结束日期包含完整当天
    pub during: Option<String>,
    /// 条目数量，默认 20，最大 100
    pub limit: Option<u64>,
}

/// 需要指定新闻来源的公开接口查询参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsSourceQuery {
    /// 新闻来源 ID，例如 web_cn 或 mys
    pub source_id: String,
}

/// 视频新闻单集 NFO 接口的上下文参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsEpisodeNfoQuery {
    /// 新闻来源 ID，例如 web_cn 或 mys
    pub source_id: String,
    /// 季编号，0 表示特别篇
    pub season: u32,
    /// 集编号，从 1 开始
    pub episode: u32,
}

impl NewsListQuery {
    /// 将 HTTP 专属筛选条件解析为应用层新闻列表命令
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
            query: self.q,
            tags: parse_csv(self.tags),
            news_type: parse_news_type(self.news_type)?,
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

/// 校验并规范化公开接口的新闻类型筛选值
fn parse_news_type(value: Option<String>) -> Result<Option<String>, AppError> {
    let value = value.map(|value| value.trim().to_owned());
    match value.as_deref() {
        None | Some("") => Ok(None),
        Some("article" | "video") => Ok(value),
        Some(_) => Err(AppError::BadRequest(
            "news_type must be article or video".to_owned(),
        )),
    }
}

impl NewsRssQuery {
    /// 将 RSS 筛选参数转换为固定倒序且不分页偏移的应用层查询
    pub(super) fn into_filter(self, path: GamePath) -> Result<ListNewsFilter, AppError> {
        let Self {
            source_id,
            q,
            tags,
            news_type,
            during,
            limit,
        } = self;

        NewsListQuery {
            source_id,
            q,
            tags,
            news_type,
            during,
            limit,
            offset: Some(0),
            reverse: Some(false),
        }
        .into_filter(path)
    }
}

impl NewsEpisodeNfoQuery {
    /// 校验媒体库文件名和 NFO 使用的季集编号
    pub(super) fn validate(self) -> Result<Self, AppError> {
        if self.season > 9_999 {
            return Err(AppError::BadRequest(
                "season must be between 0 and 9999".to_owned(),
            ));
        }
        if !(1..=999_999).contains(&self.episode) {
            return Err(AppError::BadRequest(
                "episode must be between 1 and 999999".to_owned(),
            ));
        }

        Ok(self)
    }
}

/// 将逗号分隔的查询值拆分为非空筛选项
fn parse_csv(value: Option<String>) -> Option<Vec<String>> {
    let items = value?
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    (!items.is_empty()).then_some(items)
}

/// 将中国时区日期区间解析为右开时间范围
fn parse_time_interval(during: &str) -> Result<PublishTimeRange, String> {
    /// 解析一个紧凑格式的 YYYYMMDD 日期值
    fn parse_date(value: &str) -> Result<NaiveDate, String> {
        if value.len() != 8 || !value.chars().all(|ch| ch.is_ascii_digit()) {
            return Err("日期格式应为 YYYYMMDD".to_string());
        }

        NaiveDate::parse_from_str(value, "%Y%m%d").map_err(|_| "日期无效".to_string())
    }

    /// 将日历日期转换为配置固定时区中的当天起点
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

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use crate::http::error::AppError;

    use super::{NewsEpisodeNfoQuery, parse_news_type, parse_time_interval};

    /// 新闻类型筛选只接受公开支持的两个值
    #[test]
    fn validates_news_type() {
        assert_eq!(
            parse_news_type(Some(" video ".to_owned())).expect("应接受视频类型"),
            Some("video".to_owned())
        );
        assert!(matches!(
            parse_news_type(Some("unknown".to_owned())),
            Err(AppError::BadRequest(_))
        ));
    }

    /// 日期范围的结束日期按完整自然日计算
    #[test]
    fn parses_inclusive_calendar_date_range() {
        let (start, end) = parse_time_interval("20260801-20260802").expect("应解析日期范围");

        assert_eq!(start.expect("应包含开始时间").day(), 1);
        assert_eq!(end.expect("应包含结束时间").day(), 3);
    }

    /// 单集 NFO 只接受媒体库可用的季集编号
    #[test]
    fn validates_episode_numbers() {
        assert!(
            NewsEpisodeNfoQuery {
                source_id: "web_cn".to_owned(),
                season: 0,
                episode: 1,
            }
            .validate()
            .is_ok()
        );
        assert!(matches!(
            NewsEpisodeNfoQuery {
                source_id: "web_cn".to_owned(),
                season: 1,
                episode: 0,
            }
            .validate(),
            Err(AppError::BadRequest(_))
        ));
    }
}
