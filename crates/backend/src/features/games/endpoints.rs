use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    features::games::{
        dto::{GameDetailResponse, GameResponse},
        use_cases,
    },
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
pub(super) async fn list(
    State(state): State<AppState>,
) -> Result<Json<ListResponse<GameResponse>>, AppError> {
    let rows = use_cases::list(state.db())
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    let items = rows.into_iter().map(GameResponse::from).collect::<Vec<_>>();

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
        (status = 200, body = GameDetailResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn detail(
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
) -> Result<Json<GameDetailResponse>, AppError> {
    let game = use_cases::detail(state.db(), &game_id)
        .await
        .map_err(|error| AppError::Internal(error.into()))?
        .ok_or_else(|| AppError::NotFound(format!("game {game_id} not found")))?;

    Ok(Json(GameDetailResponse::from(game)))
}
