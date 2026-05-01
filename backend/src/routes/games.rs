use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
};
use db::entities::{games, news_items};
use sea_orm::{ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use axum_mcp::{MCPInputSchema, mcp};

pub fn router() -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(games))
        .routes(routes!(game))
}

#[mcp]
#[utoipa::path(
    get,
    path = "/games",
    tag = "Games",
    summary = "获取游戏列表",
    description = "返回当前数据库中已收录的游戏基础信息；传入 is_video=true 时只返回有视频新闻的游戏，传入 is_video=false 时只返回有非视频新闻的游戏。",
    params(GamesQuery),
    responses(
        (status = 200, body = GamesResponse),
        (status = 500, body = GameErrorResponse)
    )
)]
pub async fn games(
    Query(query): Query<GamesQuery>,
) -> Result<Json<GamesResponse>, (StatusCode, Json<GameErrorResponse>)> {
    let games = games::Entity::find()
        .order_by(games::Column::Index, Order::Asc)
        .all(db::pool())
        .await
        .map_err(internal_error)?;

    let mut summaries = Vec::new();

    for game in games {
        let summary = GameSummary::from_model(game, query.is_video)
            .await
            .map_err(internal_error)?;

        if query.is_video.is_some() && summary.news_count == 0 {
            continue;
        }

        summaries.push(summary);
    }

    Ok(Json(GamesResponse { games: summaries }))
}

#[mcp]
#[utoipa::path(
    get,
    path = "/games/{game_code}",
    tag = "Games",
    summary = "获取游戏详情",
    description = "根据游戏代码或中英文名称获取单个游戏的基础信息。",
    params(GamePath),
    responses(
        (status = 200, body = GameResponse),
        (status = 404, body = GameErrorResponse),
        (status = 500, body = GameErrorResponse)
    )
)]
pub async fn game(
    Path(path): Path<GamePath>,
) -> Result<Json<GameResponse>, (StatusCode, Json<GameErrorResponse>)> {
    let Some(game_code) = games::Entity::resolve_game_code(&path.game_code)
        .await
        .map_err(internal_error)?
    else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(GameErrorResponse {
                message: "game not found".to_string(),
            }),
        ));
    };

    let game = games::Entity::find_by_id(game_code)
        .one(db::pool())
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(GameErrorResponse {
                    message: "game not found".to_string(),
                }),
            )
        })?;

    Ok(Json(GameResponse {
        game: GameSummary::from_model(game, None)
            .await
            .map_err(internal_error)?,
    }))
}

fn internal_error(err: sea_orm::DbErr) -> (StatusCode, Json<GameErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(GameErrorResponse {
            message: err.to_string(),
        }),
    )
}

#[derive(Deserialize, IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Query)]
pub struct GamesQuery {
    /// 按新闻视频类型筛选游戏；true 表示只返回有视频新闻的游戏，false 表示只返回有非视频新闻的游戏。
    is_video: Option<bool>,
}

#[derive(serde::Deserialize, utoipa::IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Path)]
pub struct GamePath {
    /// 游戏代码或中英文名称，例如 hk4e、原神、Genshin Impact。
    game_code: String,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "游戏基础信息。")]
pub struct GameSummary {
    /// 游戏代码。
    game_code: String,
    /// 英文名称。
    name_en: String,
    /// 中文名称。
    name_zh: String,
    /// 展示排序。
    index: u32,
    /// 封面图片地址。
    cover: String,
    /// 扩展信息。
    extra: Option<String>,
    /// 当前游戏关联的新闻数量；传入 is_video 时按该视频类型计数。
    news_count: u64,
}

impl GameSummary {
    async fn from_model(
        game: games::Model,
        is_video: Option<bool>,
    ) -> Result<Self, sea_orm::DbErr> {
        let mut query =
            news_items::Entity::find().filter(news_items::Column::GameCode.eq(&game.game_code));

        if let Some(is_video) = is_video {
            query = query.filter(news_items::Column::IsVideo.eq(is_video));
        }

        let news_count = query.count(db::pool()).await?;

        Ok(Self {
            game_code: game.game_code,
            name_en: game.name_en,
            name_zh: game.name_zh,
            index: game.index,
            cover: game.cover,
            extra: game.extra,
            news_count,
        })
    }
}

#[derive(Serialize, ToSchema)]
#[schema(description = "游戏列表响应。")]
pub struct GamesResponse {
    /// 游戏列表。
    games: Vec<GameSummary>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "游戏详情响应。")]
pub struct GameResponse {
    /// 游戏详情。
    game: GameSummary,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "游戏接口错误响应。")]
pub struct GameErrorResponse {
    /// 错误信息。
    message: String,
}
