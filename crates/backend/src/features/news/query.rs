use std::collections::HashSet;

use akasha_application::{
    news::{ListNewsFilter, NewsFeedFilter, NewsFilter, NewsOrder},
    search::TextQuery,
};
use chrono::{DateTime, Days, FixedOffset, NaiveDate, TimeZone};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{http::error::AppError, http::path::GamePath};

const DEFAULT_PAGE_LIMIT: u64 = 20;
const MAX_PAGE_LIMIT: u64 = 100;
const MAX_RELATION_FILTERS: usize = 32;
const MAX_RELATION_FILTER_CHARS: usize = 100;

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

/// 新闻列表与 RSS 共用的筛选参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsFilterQuery {
    /// 新闻来源，例如 web_cn 或 mys
    pub source: String,
    /// 标题查询，支持空格 AND、竖线 OR、减号排除、引号短语和反斜杠转义
    pub q: Option<String>,
    /// 任一匹配标签，可重复传入
    #[serde(default)]
    pub tag: Vec<String>,
    /// 是否同时包含没有任何标签的新闻
    pub untagged: Option<bool>,
    /// 任一匹配角色 ID，可重复传入
    #[serde(default)]
    pub character: Vec<String>,
    /// 新闻类型，仅支持 article 或 video
    pub news_type: Option<String>,
    /// 发布日期下界，格式为 YYYY-MM-DD，包含当天
    pub published_from: Option<String>,
    /// 发布日期上界，格式为 YYYY-MM-DD，包含当天
    pub published_to: Option<String>,
}

/// 新闻列表分页和排序参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsPageQuery {
    /// 每页数量，默认 20，最大 100
    pub limit: Option<u64>,
    /// 分页偏移，默认 0
    pub offset: Option<u64>,
    /// 发布时间顺序，仅支持 asc 或 desc，默认 desc
    pub order: Option<String>,
}

/// RSS 条目数量参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsRssQuery {
    /// 条目数量，默认 20，最大 100
    pub limit: Option<u64>,
}

/// 需要指定新闻来源的公开接口查询参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsSourceQuery {
    /// 新闻来源，例如 web_cn 或 mys
    pub source: String,
}

impl NewsSourceQuery {
    /// 校验并返回公开新闻来源值
    pub(super) fn into_source(self) -> Result<String, AppError> {
        required_value(self.source, "source")
    }
}

/// 视频新闻单集 NFO 接受的上下文参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NewsEpisodeNfoQuery {
    /// 新闻来源，例如 web_cn 或 mys
    pub source: String,
    /// 季编号，0 表示特别篇
    pub season: u32,
    /// 集编号，从 1 开始
    pub episode: u32,
}

impl NewsFilterQuery {
    /// 将 HTTP 筛选参数解析为列表与 RSS 共用的应用层条件
    pub(super) fn into_filter(
        self,
        GamePath { game_id }: GamePath,
    ) -> Result<NewsFilter, AppError> {
        let (start_publish_time, end_publish_time) =
            parse_publish_time_range(self.published_from, self.published_to)
                .map_err(AppError::BadRequest)?;
        let title_query = self
            .q
            .as_deref()
            .map(TextQuery::parse)
            .transpose()
            .map_err(|error| AppError::BadRequest(error.to_string()))?
            .filter(|query| !query.is_empty());

        Ok(NewsFilter {
            source_id: required_value(self.source, "source")?,
            game_id,
            title_query,
            tags: normalize_relation_filters(self.tag, "tag")?,
            include_untagged: self.untagged.unwrap_or(false),
            character_ids: normalize_relation_filters(self.character, "character")?,
            news_type: parse_news_type(self.news_type)?,
            start_publish_time,
            end_publish_time,
        })
    }
}

impl NewsPageQuery {
    /// 为公共列表筛选条件附加分页和排序
    pub(super) fn apply(self, filter: NewsFilter) -> Result<ListNewsFilter, AppError> {
        let order = match self.order.as_deref().map(str::trim) {
            None | Some("") | Some("desc") => NewsOrder::Desc,
            Some("asc") => NewsOrder::Asc,
            Some(_) => {
                return Err(AppError::BadRequest("order must be asc or desc".to_owned()));
            }
        };

        Ok(ListNewsFilter {
            filter,
            limit: self
                .limit
                .unwrap_or(DEFAULT_PAGE_LIMIT)
                .clamp(1, MAX_PAGE_LIMIT),
            offset: self.offset.unwrap_or(0),
            order,
        })
    }
}

impl NewsRssQuery {
    /// 为公共筛选条件附加固定倒序 RSS 条目数量
    pub(super) fn apply(self, filter: NewsFilter) -> NewsFeedFilter {
        NewsFeedFilter {
            filter,
            limit: self
                .limit
                .unwrap_or(DEFAULT_PAGE_LIMIT)
                .clamp(1, MAX_PAGE_LIMIT),
        }
    }
}

impl NewsEpisodeNfoQuery {
    /// 校验媒体库文件名和 NFO 使用的季集编号
    pub(super) fn validate(self) -> Result<Self, AppError> {
        if self.source.trim().is_empty() {
            return Err(AppError::BadRequest("source must not be empty".to_owned()));
        }
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

        Ok(Self {
            source: self.source.trim().to_owned(),
            ..self
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

/// 规范化并限制可重复的关系筛选值
fn normalize_relation_filters(
    values: Vec<String>,
    field_name: &str,
) -> Result<Vec<String>, AppError> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > MAX_RELATION_FILTER_CHARS {
            return Err(AppError::BadRequest(format!(
                "{field_name} values must not exceed {MAX_RELATION_FILTER_CHARS} characters"
            )));
        }
        if seen.insert(value.to_owned()) {
            normalized.push(value.to_owned());
        }
    }

    if normalized.len() > MAX_RELATION_FILTERS {
        return Err(AppError::BadRequest(format!(
            "{field_name} must not contain more than {MAX_RELATION_FILTERS} values"
        )));
    }
    Ok(normalized)
}

/// 校验必填查询字符串并去除首尾空白
fn required_value(value: String, field_name: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!(
            "{field_name} must not be empty"
        )));
    }
    Ok(value.to_owned())
}

/// 将公开日期上下界解析为中国时区中的右开时间范围
fn parse_publish_time_range(
    from: Option<String>,
    to: Option<String>,
) -> Result<PublishTimeRange, String> {
    let timezone = FixedOffset::east_opt(8 * 60 * 60).expect("valid fixed timezone");
    let start = from
        .as_deref()
        .map(|value| parse_date(value, "published_from"))
        .transpose()?
        .map(|date| at_day_start(date, timezone))
        .transpose()?;
    let end = to
        .as_deref()
        .map(|value| parse_date(value, "published_to"))
        .transpose()?
        .map(|date| {
            date.checked_add_days(Days::new(1))
                .ok_or_else(|| "published_to is outside the supported range".to_owned())
                .and_then(|date| at_day_start(date, timezone))
        })
        .transpose()?;

    if let (Some(start), Some(end)) = (start, end)
        && start >= end
    {
        return Err("published_from must be earlier than or equal to published_to".to_owned());
    }
    Ok((start, end))
}

/// 解析一个公开 YYYY-MM-DD 日期参数
fn parse_date(value: &str, field_name: &str) -> Result<NaiveDate, String> {
    let value = value.trim();
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| format!("{field_name} must use YYYY-MM-DD format"))
}

/// 将日历日期转换为配置固定时区中的当天起点
fn at_day_start(date: NaiveDate, timezone: FixedOffset) -> Result<DateTime<FixedOffset>, String> {
    timezone
        .from_local_datetime(
            &date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| "date is outside the supported range".to_owned())?,
        )
        .single()
        .ok_or_else(|| "date is outside the supported range".to_owned())
}

#[cfg(test)]
mod tests {
    use axum::{extract::Query as AxumQuery, http::Uri};
    use axum_extra::extract::Query;
    use chrono::Datelike;

    use crate::http::error::AppError;

    use super::{
        NewsEpisodeNfoQuery, NewsFilterQuery, NewsPageQuery, normalize_relation_filters,
        parse_news_type, parse_publish_time_range,
    };

    #[test]
    fn validates_news_type_and_order() {
        assert_eq!(
            parse_news_type(Some(" video ".to_owned())).expect("应接受视频类型"),
            Some("video".to_owned())
        );
        assert!(matches!(
            parse_news_type(Some("unknown".to_owned())),
            Err(AppError::BadRequest(_))
        ));
        let filter = akasha_application::news::NewsFilter {
            source_id: "mys".to_owned(),
            game_id: "ys".to_owned(),
            title_query: None,
            tags: Vec::new(),
            include_untagged: false,
            character_ids: Vec::new(),
            news_type: None,
            start_publish_time: None,
            end_publish_time: None,
        };
        assert!(
            NewsPageQuery {
                limit: None,
                offset: None,
                order: Some("unknown".to_owned()),
            }
            .apply(filter)
            .is_err()
        );
    }

    #[test]
    fn parses_inclusive_calendar_date_range() {
        let (start, end) =
            parse_publish_time_range(Some("2026-08-01".to_owned()), Some("2026-08-02".to_owned()))
                .expect("应解析日期范围");

        assert_eq!(start.expect("应包含开始时间").day(), 1);
        assert_eq!(end.expect("应包含结束时间").day(), 3);
    }

    #[test]
    fn normalizes_repeated_relation_filters() {
        assert_eq!(
            normalize_relation_filters(
                vec![
                    " 10000046 ".to_owned(),
                    "10000046".to_owned(),
                    "".to_owned(),
                ],
                "character",
            )
            .expect("应规范化角色筛选"),
            ["10000046"]
        );
    }

    #[test]
    fn deserializes_repeated_public_filters() {
        let uri: Uri =
            "/api/v1/games/ys/news?source=mys&tag=角色&tag=活动&character=10000046&character=10000073"
                .parse()
                .expect("应构造查询 URI");
        let Query(query) =
            Query::<NewsFilterQuery>::try_from_uri(&uri).expect("应解析重复的标签和角色参数");
        let AxumQuery(page) =
            AxumQuery::<NewsPageQuery>::try_from_uri(&uri).expect("分页参数解析应忽略共用筛选参数");

        assert_eq!(query.source, "mys");
        assert_eq!(query.tag, ["角色", "活动"]);
        assert_eq!(query.character, ["10000046", "10000073"]);
        assert_eq!(page.limit, None);
    }

    #[test]
    fn validates_episode_numbers_and_source() {
        assert!(
            NewsEpisodeNfoQuery {
                source: " web_cn ".to_owned(),
                season: 0,
                episode: 1,
            }
            .validate()
            .is_ok()
        );
        assert!(matches!(
            NewsEpisodeNfoQuery {
                source: "web_cn".to_owned(),
                season: 1,
                episode: 0,
            }
            .validate(),
            Err(AppError::BadRequest(_))
        ));
    }
}
