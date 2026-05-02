use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
};
use chrono::{DateTime, FixedOffset};
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
        .routes(routes!(character))
}

#[mcp]
#[utoipa::path(
    get,
    path = "/characters",
    tag = "Characters",
    summary = "获取角色列表",
    description = "按游戏、性别和关键词筛选角色，返回分页后的角色基础信息。",
    params(CharactersQuery),
    responses(
        (status = 200, body = CharactersResponse),
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

    if let Some(keyword) = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        statement = statement.filter(
            Condition::any()
                .add(game_characters::Column::CharacterCode.contains(keyword))
                .add(game_characters::Column::Name.contains(keyword)),
        );
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

#[derive(Deserialize, IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Query)]
pub struct CharactersQuery {
    /// 游戏代码或中英文名称，例如 hk4e、原神、Genshin Impact。
    game: Option<String>,
    /// 性别筛选，支持 male、female、unknown，也支持男、女、未知。
    gender: Option<String>,
    /// 按角色代码或角色名搜索。
    q: Option<String>,
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
    birthday_month: Option<u8>,
    /// 生日日期。
    birthday_day: Option<u8>,
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
