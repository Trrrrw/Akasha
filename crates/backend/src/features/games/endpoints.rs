use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    features::games::dto::GameResponse,
    http::{
        error::AppError,
        path::GamePath,
        response::{ErrorResponse, ListResponse},
    },
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/games",
    tag = "Game",
    summary = "获取游戏列表",
    description = "返回当前数据库中已收录的游戏基础信息",
    responses(
        (status = 200, body = ListResponse<GameResponse>),
        (status = 500, body = ErrorResponse)
    )
)]
/// 列出包含公开展示元数据的全部游戏
pub(super) async fn list(
    State(state): State<AppState>,
) -> Result<Json<ListResponse<GameResponse>>, AppError> {
    let rows = state.application().list_games().await?;
    let mut items = Vec::with_capacity(rows.len());
    for game in rows {
        items.push(GameResponse::from_summary(
            game,
            &state.config().asset_base_url,
        ));
    }

    Ok(Json(ListResponse {
        total: items.len() as u64,
        items,
    }))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}",
    tag = "Game",
    summary = "获取游戏详情",
    description = "返回指定游戏详情信息",
    params(GamePath),
    responses(
        (status = 200, body = GameResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
/// 返回请求游戏，未找到时返回 not found 响应
pub(super) async fn detail(
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
) -> Result<Json<GameResponse>, AppError> {
    let game = state
        .application()
        .find_game(&game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("game {game_id} not found")))?;

    Ok(Json(GameResponse::from_summary(
        game,
        &state.config().asset_base_url,
    )))
}
