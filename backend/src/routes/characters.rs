use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
};
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, TimeZone};
use db::entities::{game_characters, games};
use sea_orm::{
    ActiveEnum, ColumnTrait, Condition, EntityTrait, Order, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use axum_mcp::{MCPInputSchema, mcp};

const DEFAULT_PAGE: u64 = 1;
const DEFAULT_PAGE_SIZE: u64 = 20;
const MAX_PAGE_SIZE: u64 = 100;

pub fn router() -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(characters))
        .routes(routes!(character_search))
        .routes(routes!(character_stats))
        .routes(routes!(character))
}

#[mcp]
#[utoipa::path(
    get,
    path = "/characters",
    tag = "Characters",
    summary = "获取角色列表",
    description = "按游戏、性别和生日筛选角色，返回分页后的角色基础信息。",
    params(CharactersQuery),
    responses(
        (status = 200, body = CharactersResponse),
        (status = 400, body = CharacterErrorResponse),
        (status = 500, body = CharacterErrorResponse)
    )
)]
pub async fn characters(
    Query(query): Query<CharactersQuery>,
) -> Result<Json<CharactersResponse>, (StatusCode, Json<CharacterErrorResponse>)> {
    let conn = db::pool();
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let mut statement = game_characters::Entity::find();

    if let Some(game_code) = resolve_optional_game(query.game.as_deref())
        .await
        .map_err(internal_error)?
    {
        statement = statement.filter(game_characters::Column::GameCode.eq(game_code));
    } else if query.game.is_some() {
        return Ok(Json(CharactersResponse {
            page,
            page_size,
            total: 0,
            characters: Vec::new(),
        }));
    }

    if let Some(gender) = query.gender.as_deref().and_then(normalize_gender) {
        statement = statement.filter(game_characters::Column::Gender.eq(gender));
    } else if query.gender.is_some() {
        return Ok(Json(CharactersResponse {
            page,
            page_size,
            total: 0,
            characters: Vec::new(),
        }));
    }

    if let Some(name) = query
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        statement = statement.filter(game_characters::Column::Name.contains(name));
    }

    if let Some(cv) = query
        .cv
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        statement = statement.filter(game_characters::Column::Cv.contains(cv));
    }

    statement = apply_birthday_filter(statement, &query)?;
    statement = apply_release_filter(statement, query.release.as_deref())?;

    let total = statement
        .clone()
        .count(conn)
        .await
        .map_err(internal_error)?;
    let offset = (page - 1) * page_size;
    let characters = statement
        .order_by(game_characters::Column::GameCode, Order::Asc)
        .order_by(game_characters::Column::Name, Order::Asc)
        .offset(offset)
        .limit(page_size)
        .find_also_related(games::Entity)
        .all(conn)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|(character, game)| CharacterSummary::from_models(character, game))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(CharactersResponse {
        page,
        page_size,
        total,
        characters,
    }))
}

#[mcp]
#[utoipa::path(
    get,
    path = "/characters/search",
    tag = "Characters",
    summary = "按角色名搜索角色",
    description = "按角色名关键词搜索角色，重名角色会以列表形式返回。",
    params(CharacterSearchQuery),
    responses(
        (status = 200, body = CharactersResponse),
        (status = 400, body = CharacterErrorResponse),
        (status = 500, body = CharacterErrorResponse)
    )
)]
pub async fn character_search(
    Query(query): Query<CharacterSearchQuery>,
) -> Result<Json<CharactersResponse>, (StatusCode, Json<CharacterErrorResponse>)> {
    let conn = db::pool();
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let Some(name) = query
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    else {
        return Err(bad_request("name is required"));
    };

    let mut statement =
        game_characters::Entity::find().filter(game_characters::Column::Name.contains(name));

    if let Some(game_code) = resolve_optional_game(query.game.as_deref())
        .await
        .map_err(internal_error)?
    {
        statement = statement.filter(game_characters::Column::GameCode.eq(game_code));
    } else if query.game.is_some() {
        return Ok(Json(CharactersResponse {
            page,
            page_size,
            total: 0,
            characters: Vec::new(),
        }));
    }

    let total = statement
        .clone()
        .count(conn)
        .await
        .map_err(internal_error)?;
    let offset = (page - 1) * page_size;
    let characters = statement
        .order_by(game_characters::Column::GameCode, Order::Asc)
        .order_by(game_characters::Column::Name, Order::Asc)
        .offset(offset)
        .limit(page_size)
        .find_also_related(games::Entity)
        .all(conn)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|(character, game)| CharacterSummary::from_models(character, game))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(CharactersResponse {
        page,
        page_size,
        total,
        characters,
    }))
}

#[mcp]
#[utoipa::path(
    get,
    path = "/characters/stats",
    tag = "Characters",
    summary = "统计角色数据",
    description = "按游戏、性别、生日、发布时间或 CV 聚合统计角色数量和占比。用户询问角色数量、比例或分布时优先使用此接口。",
    params(CharacterStatsQuery),
    responses(
        (status = 200, body = CharacterStatsResponse),
        (status = 400, body = CharacterErrorResponse),
        (status = 500, body = CharacterErrorResponse)
    )
)]
pub async fn character_stats(
    Query(query): Query<CharacterStatsQuery>,
) -> Result<Json<CharacterStatsResponse>, (StatusCode, Json<CharacterErrorResponse>)> {
    let group_by = query
        .group_by
        .as_deref()
        .map(CharacterStatsGroupBy::parse)
        .transpose()?
        .unwrap_or(CharacterStatsGroupBy::Game);
    let mut statement = game_characters::Entity::find();

    if let Some(game_code) = resolve_optional_game(query.game.as_deref())
        .await
        .map_err(internal_error)?
    {
        statement = statement.filter(game_characters::Column::GameCode.eq(game_code));
    } else if query.game.is_some() {
        return Ok(Json(CharacterStatsResponse::empty(group_by)));
    }

    if let Some(gender) = query.gender.as_deref().and_then(normalize_gender) {
        statement = statement.filter(game_characters::Column::Gender.eq(gender));
    } else if query.gender.is_some() {
        return Ok(Json(CharacterStatsResponse::empty(group_by)));
    }

    statement = apply_birthday_filter_value(statement, query.birthday.as_deref())?;
    statement = apply_release_filter(statement, query.release.as_deref())?;

    let rows = statement
        .find_also_related(games::Entity)
        .all(db::pool())
        .await
        .map_err(internal_error)?;

    let total = rows.len() as u64;
    let mut groups = BTreeMap::<String, CharacterStatsAccumulator>::new();

    for (character, game) in rows {
        let group = CharacterStatsGroupKey::from_models(group_by, &character, game)?;
        let entry = groups
            .entry(group.key)
            .or_insert(CharacterStatsAccumulator {
                label: group.label,
                game: group.game,
                count: 0,
            });
        entry.count += 1;
    }

    let groups = groups
        .into_iter()
        .map(|(key, group)| CharacterStatsGroup {
            key,
            label: group.label,
            count: group.count,
            percent: percent(group.count, total),
            game: group.game,
        })
        .collect();

    Ok(Json(CharacterStatsResponse {
        group_by: group_by.as_str().to_string(),
        total,
        groups,
    }))
}

#[mcp]
#[utoipa::path(
    get,
    path = "/characters/{game_code}/{character_code}",
    tag = "Characters",
    summary = "获取角色详情",
    description = "根据游戏代码和角色代码获取单个角色的详情信息；游戏参数支持代码或中英文名称。",
    params(CharacterPath),
    responses(
        (status = 200, body = CharacterResponse),
        (status = 404, body = CharacterErrorResponse),
        (status = 500, body = CharacterErrorResponse)
    )
)]
pub async fn character(
    Path(path): Path<CharacterPath>,
) -> Result<Json<CharacterResponse>, (StatusCode, Json<CharacterErrorResponse>)> {
    let Some(game_code) = games::Entity::resolve_game_code(&path.game_code)
        .await
        .map_err(internal_error)?
    else {
        return Err(not_found());
    };

    let (character, game) = game_characters::Entity::find_by_id((path.character_code, game_code))
        .find_also_related(games::Entity)
        .one(db::pool())
        .await
        .map_err(internal_error)?
        .ok_or_else(not_found)?;

    Ok(Json(CharacterResponse {
        character: CharacterDetail::from_models(character, game)?,
    }))
}

fn internal_error(err: sea_orm::DbErr) -> (StatusCode, Json<CharacterErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(CharacterErrorResponse {
            message: err.to_string(),
        }),
    )
}

fn not_found() -> (StatusCode, Json<CharacterErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(CharacterErrorResponse {
            message: "character not found".to_string(),
        }),
    )
}

fn bad_request(message: impl Into<String>) -> (StatusCode, Json<CharacterErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(CharacterErrorResponse {
            message: message.into(),
        }),
    )
}

async fn resolve_optional_game(input: Option<&str>) -> Result<Option<String>, sea_orm::DbErr> {
    match input {
        Some(value) => games::Entity::resolve_game_code(value).await,
        None => Ok(None),
    }
}

fn normalize_gender(value: &str) -> Option<String> {
    match value.trim().to_lowercase().as_str() {
        "male" | "m" | "男" => Some("male".to_string()),
        "female" | "f" | "女" => Some("female".to_string()),
        "unknown" | "未知" => Some("unknown".to_string()),
        _ => None,
    }
}

fn format_datetime(value: DateTime<FixedOffset>) -> String {
    value.to_rfc3339()
}

fn apply_birthday_filter(
    statement: sea_orm::Select<game_characters::Entity>,
    query: &CharactersQuery,
) -> Result<sea_orm::Select<game_characters::Entity>, (StatusCode, Json<CharacterErrorResponse>)> {
    apply_birthday_filter_value(statement, query.birthday.as_deref())
}

fn apply_birthday_filter_value(
    statement: sea_orm::Select<game_characters::Entity>,
    birthday: Option<&str>,
) -> Result<sea_orm::Select<game_characters::Entity>, (StatusCode, Json<CharacterErrorResponse>)> {
    let Some(birthday) = birthday.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(statement);
    };

    match parse_birthday_filter(birthday)? {
        BirthdayFilter::Month(month) => {
            Ok(statement.filter(game_characters::Column::BirthdayMonth.eq(month)))
        }
        BirthdayFilter::Date(date) => Ok(statement.filter(
            Condition::all()
                .add(game_characters::Column::BirthdayMonth.eq(date.month))
                .add(game_characters::Column::BirthdayDay.eq(date.day)),
        )),
        BirthdayFilter::Range { start, end } => {
            let condition = if start.value <= end.value {
                Condition::all()
                    .add(on_or_after_birthday(start))
                    .add(on_or_before_birthday(end))
            } else {
                Condition::any()
                    .add(on_or_after_birthday(start))
                    .add(on_or_before_birthday(end))
            };

            Ok(statement.filter(condition))
        }
    }
}

fn apply_release_filter(
    statement: sea_orm::Select<game_characters::Entity>,
    release: Option<&str>,
) -> Result<sea_orm::Select<game_characters::Entity>, (StatusCode, Json<CharacterErrorResponse>)> {
    let Some(release) = release.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(statement);
    };
    let range = parse_release_filter(release)?;

    Ok(statement.filter(
        Condition::all()
            .add(game_characters::Column::ReleaseTime.gte(range.start))
            .add(game_characters::Column::ReleaseTime.lt(range.end)),
    ))
}

fn parse_release_filter(
    value: &str,
) -> Result<ReleaseDateRange, (StatusCode, Json<CharacterErrorResponse>)> {
    let parts = value.split('-').collect::<Vec<_>>();
    let (year, month, day) = match parts.as_slice() {
        [year] => (parse_release_part(year, "release year")?, None, None),
        [year, month] => (
            parse_release_part(year, "release year")?,
            Some(parse_release_part(month, "release month")?),
            None,
        ),
        [year, month, day] => (
            parse_release_part(year, "release year")?,
            Some(parse_release_part(month, "release month")?),
            Some(parse_release_part(day, "release day")?),
        ),
        _ => return Err(invalid_release_format()),
    };

    if !(1..=9999).contains(&year) {
        return Err(bad_request("invalid release year"));
    }

    let start_date = match (month, day) {
        (None, None) => NaiveDate::from_ymd_opt(year, 1, 1),
        (Some(month), None) => NaiveDate::from_ymd_opt(year, month as u32, 1),
        (Some(month), Some(day)) => NaiveDate::from_ymd_opt(year, month as u32, day as u32),
        (None, Some(_)) => None,
    }
    .ok_or_else(|| bad_request("invalid release date"))?;

    let end_date = match (month, day) {
        (None, None) => NaiveDate::from_ymd_opt(year + 1, 1, 1),
        (Some(12), None) => NaiveDate::from_ymd_opt(year + 1, 1, 1),
        (Some(month), None) => NaiveDate::from_ymd_opt(year, month as u32 + 1, 1),
        (_, Some(_)) => start_date.succ_opt(),
    }
    .ok_or_else(|| bad_request("invalid release date"))?;

    let offset = FixedOffset::east_opt(8 * 3600).unwrap();
    let start = offset
        .from_local_datetime(&start_date.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .ok_or_else(|| bad_request("invalid release date"))?;
    let end = offset
        .from_local_datetime(&end_date.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .ok_or_else(|| bad_request("invalid release date"))?;

    Ok(ReleaseDateRange { start, end })
}

fn parse_release_part(
    value: &str,
    name: &str,
) -> Result<i32, (StatusCode, Json<CharacterErrorResponse>)> {
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(bad_request(format!("{name} must be numeric")));
    }

    value
        .parse()
        .map_err(|_| bad_request(format!("invalid {name}")))
}

fn invalid_release_format() -> (StatusCode, Json<CharacterErrorResponse>) {
    bad_request("release must use YYYY, YYYY-MM, or YYYY-MM-DD format")
}

fn on_or_after_birthday(date: BirthdayDate) -> Condition {
    Condition::any()
        .add(game_characters::Column::BirthdayMonth.gt(date.month))
        .add(
            Condition::all()
                .add(game_characters::Column::BirthdayMonth.eq(date.month))
                .add(game_characters::Column::BirthdayDay.gte(date.day)),
        )
}

fn on_or_before_birthday(date: BirthdayDate) -> Condition {
    Condition::any()
        .add(game_characters::Column::BirthdayMonth.lt(date.month))
        .add(
            Condition::all()
                .add(game_characters::Column::BirthdayMonth.eq(date.month))
                .add(game_characters::Column::BirthdayDay.lte(date.day)),
        )
}

fn parse_birthday_filter(
    value: &str,
) -> Result<BirthdayFilter, (StatusCode, Json<CharacterErrorResponse>)> {
    if let Some((start, end)) = value.split_once('-') {
        let start = parse_mmdd(start)?;
        let end = parse_mmdd(end)?;
        return Ok(BirthdayFilter::Range { start, end });
    }

    if value.len() == 2 {
        return Ok(BirthdayFilter::Month(parse_month(value)?));
    }

    Ok(BirthdayFilter::Date(parse_mmdd(value)?))
}

fn parse_month(value: &str) -> Result<i16, (StatusCode, Json<CharacterErrorResponse>)> {
    let value = value.trim();
    if value.len() != 2 || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(invalid_birthday_format());
    }

    let month = value
        .parse::<u32>()
        .map_err(|_| invalid_birthday_format())?;

    if !(1..=12).contains(&month) {
        return Err(bad_request("invalid birthday month"));
    }

    Ok(month as i16)
}

fn parse_mmdd(value: &str) -> Result<BirthdayDate, (StatusCode, Json<CharacterErrorResponse>)> {
    let value = value.trim();
    if value.len() != 4 || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(invalid_birthday_format());
    }

    let month = value[0..2]
        .parse::<u32>()
        .map_err(|_| invalid_birthday_format())?;
    let day = value[2..4]
        .parse::<u32>()
        .map_err(|_| invalid_birthday_format())?;

    NaiveDate::from_ymd_opt(2000, month, day)
        .ok_or_else(|| bad_request("invalid birthday date"))?;

    Ok(BirthdayDate {
        month: month as i16,
        day: day as i16,
        value: (month * 100 + day) as i16,
    })
}

fn invalid_birthday_format() -> (StatusCode, Json<CharacterErrorResponse>) {
    bad_request("birthday must use MM, MMDD, or MMDD-MMDD format")
}

fn percent(count: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }

    (count as f64 / total as f64 * 1000.0).round() / 10.0
}

#[derive(Clone, Copy)]
enum CharacterStatsGroupBy {
    Game,
    Gender,
    BirthdayMonth,
    Birthday,
    ReleaseYear,
    ReleaseMonth,
    ReleaseDate,
    Cv,
}

impl CharacterStatsGroupBy {
    fn parse(value: &str) -> Result<Self, (StatusCode, Json<CharacterErrorResponse>)> {
        match value.trim().to_lowercase().as_str() {
            "game" => Ok(Self::Game),
            "gender" => Ok(Self::Gender),
            "birthday_month" => Ok(Self::BirthdayMonth),
            "birthday" => Ok(Self::Birthday),
            "release_year" => Ok(Self::ReleaseYear),
            "release_month" => Ok(Self::ReleaseMonth),
            "release_date" => Ok(Self::ReleaseDate),
            "cv" => Ok(Self::Cv),
            _ => Err(bad_request(
                "group_by must be one of game, gender, birthday_month, birthday, release_year, release_month, release_date, cv",
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::Gender => "gender",
            Self::BirthdayMonth => "birthday_month",
            Self::Birthday => "birthday",
            Self::ReleaseYear => "release_year",
            Self::ReleaseMonth => "release_month",
            Self::ReleaseDate => "release_date",
            Self::Cv => "cv",
        }
    }
}

struct CharacterStatsGroupKey {
    key: String,
    label: String,
    game: Option<GameRef>,
}

impl CharacterStatsGroupKey {
    fn from_models(
        group_by: CharacterStatsGroupBy,
        character: &game_characters::Model,
        game: Option<games::Model>,
    ) -> Result<Self, (StatusCode, Json<CharacterErrorResponse>)> {
        match group_by {
            CharacterStatsGroupBy::Game => {
                let game = game.ok_or_else(|| {
                    internal_error(sea_orm::DbErr::RecordNotFound(format!(
                        "game not found: {}",
                        character.game_code
                    )))
                })?;
                let key = game.game_code.clone();
                let label = game.name_zh.clone();

                Ok(Self {
                    key,
                    label,
                    game: Some(GameRef::from_model(game)),
                })
            }
            CharacterStatsGroupBy::Gender => {
                let key = character
                    .gender
                    .as_ref()
                    .map(ActiveEnum::to_value)
                    .unwrap_or_else(|| "unknown".to_string());

                Ok(Self {
                    label: gender_label(&key).to_string(),
                    key,
                    game: None,
                })
            }
            CharacterStatsGroupBy::BirthdayMonth => {
                let Some(month) = character.birthday_month else {
                    return Ok(Self {
                        key: "unknown".to_string(),
                        label: "未设置".to_string(),
                        game: None,
                    });
                };

                Ok(Self {
                    key: format!("{month:02}"),
                    label: format!("{month}月"),
                    game: None,
                })
            }
            CharacterStatsGroupBy::Birthday => {
                let (Some(month), Some(day)) = (character.birthday_month, character.birthday_day)
                else {
                    return Ok(Self {
                        key: "unknown".to_string(),
                        label: "未设置".to_string(),
                        game: None,
                    });
                };

                Ok(Self {
                    key: format!("{month:02}{day:02}"),
                    label: format!("{month}月{day}日"),
                    game: None,
                })
            }
            CharacterStatsGroupBy::ReleaseYear => {
                let Some(release_time) = character.release_time else {
                    return Ok(Self {
                        key: "unknown".to_string(),
                        label: "未设置".to_string(),
                        game: None,
                    });
                };

                let year = release_time.year();
                Ok(Self {
                    key: year.to_string(),
                    label: format!("{year}年"),
                    game: None,
                })
            }
            CharacterStatsGroupBy::ReleaseMonth => {
                let Some(release_time) = character.release_time else {
                    return Ok(Self {
                        key: "unknown".to_string(),
                        label: "未设置".to_string(),
                        game: None,
                    });
                };

                let year = release_time.year();
                let month = release_time.month();
                Ok(Self {
                    key: format!("{year}-{month:02}"),
                    label: format!("{year}年{month}月"),
                    game: None,
                })
            }
            CharacterStatsGroupBy::ReleaseDate => {
                let Some(release_time) = character.release_time else {
                    return Ok(Self {
                        key: "unknown".to_string(),
                        label: "未设置".to_string(),
                        game: None,
                    });
                };

                let year = release_time.year();
                let month = release_time.month();
                let day = release_time.day();
                Ok(Self {
                    key: format!("{year}-{month:02}-{day:02}"),
                    label: format!("{year}年{month}月{day}日"),
                    game: None,
                })
            }
            CharacterStatsGroupBy::Cv => {
                let key = character
                    .cv
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("unknown")
                    .to_string();
                let label = if key == "unknown" {
                    "未设置".to_string()
                } else {
                    key.clone()
                };

                Ok(Self {
                    key,
                    label,
                    game: None,
                })
            }
        }
    }
}

fn gender_label(value: &str) -> &'static str {
    match value {
        "male" => "男",
        "female" => "女",
        _ => "未知",
    }
}

struct CharacterStatsAccumulator {
    label: String,
    game: Option<GameRef>,
    count: u64,
}

struct ReleaseDateRange {
    start: DateTime<FixedOffset>,
    end: DateTime<FixedOffset>,
}

enum BirthdayFilter {
    Month(i16),
    Date(BirthdayDate),
    Range {
        start: BirthdayDate,
        end: BirthdayDate,
    },
}

#[derive(Clone, Copy)]
struct BirthdayDate {
    month: i16,
    day: i16,
    value: i16,
}

#[derive(Deserialize, IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Query)]
pub struct CharactersQuery {
    /// 角色名关键词，模糊匹配。
    name: Option<String>,
    /// 游戏代码或中英文名称，例如 hk4e、原神、Genshin Impact。
    game: Option<String>,
    /// 性别筛选，支持 male、female、unknown，也支持男、女、未知。
    gender: Option<String>,
    /// 生日筛选。支持月份 MM，例如 09；单日 MMDD，例如 0921；范围 MMDD-MMDD，例如 0901-0930；范围支持跨年。
    birthday: Option<String>,
    /// CV 关键词，模糊匹配。
    cv: Option<String>,
    /// 发布时间筛选。支持年份 YYYY，例如 2025；年月 YYYY-MM，例如 2025-09；日期 YYYY-MM-DD，例如 2025-09-09。
    release: Option<String>,
    /// 页码，从 1 开始。
    page: Option<u64>,
    /// 每页数量，最大 100。
    page_size: Option<u64>,
}

#[derive(Deserialize, IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Query)]
pub struct CharacterSearchQuery {
    /// 角色名关键词。
    name: Option<String>,
    /// 游戏代码或中英文名称，例如 hk4e、原神、Genshin Impact。
    game: Option<String>,
    /// 页码，从 1 开始。
    page: Option<u64>,
    /// 每页数量，最大 100。
    page_size: Option<u64>,
}

#[derive(Deserialize, IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Query)]
pub struct CharacterStatsQuery {
    /// 统计维度。支持 game、gender、birthday_month、birthday、release_year、release_month、release_date、cv。用户询问男女比例时使用 gender；询问各游戏角色数量时使用 game；询问生日分布时使用 birthday_month 或 birthday；询问发布时间分布时使用 release_year、release_month 或 release_date。
    group_by: Option<String>,
    /// 游戏筛选，支持游戏中文名、英文名或代码，例如 原神、Genshin Impact、hk4e。优先传用户原文中的游戏名。
    game: Option<String>,
    /// 性别筛选，支持 male、female、unknown，也支持男、女、未知。
    gender: Option<String>,
    /// 生日筛选。支持月份 MM，例如 09；单日 MMDD，例如 0921；范围 MMDD-MMDD，例如 0901-0930；范围支持跨年。
    birthday: Option<String>,
    /// 发布时间筛选。支持年份 YYYY，例如 2025；年月 YYYY-MM，例如 2025-09；日期 YYYY-MM-DD，例如 2025-09-09。
    release: Option<String>,
}

#[derive(Deserialize, IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Path)]
pub struct CharacterPath {
    /// 游戏代码或中英文名称，例如 hk4e、原神、Genshin Impact。
    game_code: String,
    /// 角色代码。
    character_code: String,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "角色基础信息。")]
pub struct CharacterSummary {
    /// 角色代码。
    character_code: String,
    /// 所属游戏。
    game: GameRef,
    /// 角色名。
    name: String,
    /// 生日月份。
    birthday_month: Option<i16>,
    /// 生日日期。
    birthday_day: Option<i16>,
    /// 发布时间，RFC3339 格式。
    release_time: Option<String>,
    /// 性别。
    gender: Option<String>,
    /// 角色配音信息。
    cv: Option<String>,
    /// 扩展信息。
    extra: Option<String>,
}

impl CharacterSummary {
    fn from_models(
        character: game_characters::Model,
        game: Option<games::Model>,
    ) -> Result<Self, (StatusCode, Json<CharacterErrorResponse>)> {
        Ok(Self {
            character_code: character.character_code,
            game: GameRef::from_model(game.ok_or_else(|| {
                internal_error(sea_orm::DbErr::RecordNotFound(format!(
                    "game not found: {}",
                    character.game_code
                )))
            })?),
            name: character.name,
            birthday_month: character.birthday_month,
            birthday_day: character.birthday_day,
            release_time: character.release_time.map(format_datetime),
            gender: character.gender.map(|gender| gender.to_value()),
            cv: character.cv,
            extra: character.extra,
        })
    }
}

#[derive(Clone, Serialize, ToSchema)]
#[schema(description = "游戏引用信息。")]
pub struct GameRef {
    /// 游戏代码，供程序内部定位使用。
    code: String,
    /// 游戏中文名，供展示和模型回答用户使用。
    name_zh: String,
    /// 游戏英文名。
    name_en: String,
}

impl GameRef {
    fn from_model(game: games::Model) -> Self {
        Self {
            code: game.game_code,
            name_zh: game.name_zh,
            name_en: game.name_en,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[schema(description = "角色统计分组。")]
pub struct CharacterStatsGroup {
    /// 分组键，例如 female、09、0921 或游戏代码。
    key: String,
    /// 分组展示名，例如 女、9月、9月21日或游戏中文名。
    label: String,
    /// 当前分组数量。
    count: u64,
    /// 当前分组占总数百分比，保留 1 位小数。
    percent: f64,
    /// 当 group_by=game 时返回游戏引用信息。
    #[serde(skip_serializing_if = "Option::is_none")]
    game: Option<GameRef>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "角色统计响应。")]
pub struct CharacterStatsResponse {
    /// 统计维度。
    group_by: String,
    /// 统计范围内角色总数。
    total: u64,
    /// 分组统计结果。
    groups: Vec<CharacterStatsGroup>,
}

impl CharacterStatsResponse {
    fn empty(group_by: CharacterStatsGroupBy) -> Self {
        Self {
            group_by: group_by.as_str().to_string(),
            total: 0,
            groups: Vec::new(),
        }
    }
}

#[derive(Serialize, ToSchema)]
#[schema(description = "角色详情数据。")]
pub struct CharacterDetail {
    /// 角色摘要信息。
    summary: CharacterSummary,
}

impl CharacterDetail {
    fn from_models(
        character: game_characters::Model,
        game: Option<games::Model>,
    ) -> Result<Self, (StatusCode, Json<CharacterErrorResponse>)> {
        Ok(Self {
            summary: CharacterSummary::from_models(character, game)?,
        })
    }
}

#[derive(Serialize, ToSchema)]
#[schema(description = "角色列表响应。")]
pub struct CharactersResponse {
    /// 当前页码，从 1 开始。
    page: u64,
    /// 每页数量。
    page_size: u64,
    /// 符合筛选条件的角色总数。
    total: u64,
    /// 当前页角色列表。
    characters: Vec<CharacterSummary>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "角色详情响应。")]
pub struct CharacterResponse {
    /// 角色详情。
    character: CharacterDetail,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "角色接口错误响应。")]
pub struct CharacterErrorResponse {
    /// 错误信息。
    message: String,
}
