use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
};
use chrono::{DateTime, FixedOffset, NaiveDate};
use db::entities::{game_characters, games};
use sea_orm::{
    ActiveEnum, ColumnTrait, Condition, EntityTrait, Order, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
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

    statement = apply_birthday_filter(statement, &query)?;

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
        .all(conn)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(CharacterSummary::from_model)
        .collect();

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
        .all(conn)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(CharacterSummary::from_model)
        .collect();

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

    let character = game_characters::Entity::find_by_id((path.character_code, game_code))
        .one(db::pool())
        .await
        .map_err(internal_error)?
        .ok_or_else(not_found)?;

    Ok(Json(CharacterResponse {
        character: CharacterDetail::from_model(character),
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
    if let Some(date) = query.date.as_deref() {
        let date = parse_mmdd(date)?;
        return Ok(statement.filter(
            Condition::all()
                .add(game_characters::Column::BirthdayMonth.eq(date.month))
                .add(game_characters::Column::BirthdayDay.eq(date.day)),
        ));
    }

    match (query.start.as_deref(), query.end.as_deref()) {
        (None, None) => Ok(statement),
        (Some(start), Some(end)) => {
            let start = parse_mmdd(start)?;
            let end = parse_mmdd(end)?;
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
        _ => Err(bad_request("start and end must be used together")),
    }
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

fn parse_mmdd(value: &str) -> Result<BirthdayDate, (StatusCode, Json<CharacterErrorResponse>)> {
    let value = value.trim();
    if value.len() != 4 || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(bad_request("date must use MMDD format"));
    }

    let month = value[0..2]
        .parse::<u32>()
        .map_err(|_| bad_request("date must use MMDD format"))?;
    let day = value[2..4]
        .parse::<u32>()
        .map_err(|_| bad_request("date must use MMDD format"))?;

    NaiveDate::from_ymd_opt(2000, month, day)
        .ok_or_else(|| bad_request("invalid birthday date"))?;

    Ok(BirthdayDate {
        month: month as i16,
        day: day as i16,
        value: (month * 100 + day) as i16,
    })
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
    /// 游戏代码或中英文名称，例如 hk4e、原神、Genshin Impact。
    game: Option<String>,
    /// 性别筛选，支持 male、female、unknown，也支持男、女、未知。
    gender: Option<String>,
    /// 生日筛选，格式 MMDD，例如 0921。
    date: Option<String>,
    /// 生日范围开始，格式 MMDD，例如 0901。
    start: Option<String>,
    /// 生日范围结束，格式 MMDD，例如 0930；支持跨年范围。
    end: Option<String>,
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
    /// 游戏代码。
    game_code: String,
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
    /// 扩展信息。
    extra: Option<String>,
}

impl CharacterSummary {
    fn from_model(character: game_characters::Model) -> Self {
        Self {
            character_code: character.character_code,
            game_code: character.game_code,
            name: character.name,
            birthday_month: character.birthday_month,
            birthday_day: character.birthday_day,
            release_time: character.release_time.map(format_datetime),
            gender: character.gender.map(|gender| gender.to_value()),
            extra: character.extra,
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
    fn from_model(character: game_characters::Model) -> Self {
        Self {
            summary: CharacterSummary::from_model(character),
        }
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
