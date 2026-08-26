use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    features::game_data::{
        dto::{GameDataCollectionResponse, GameDataDetailResponse, GameDataEntryResponse},
        query::GameDataListQuery,
    },
    http::{
        error::AppError,
        response::{ErrorResponse, ListResponse, PageResponse},
    },
    state::AppState,
};

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub(super) struct GameDataCollectionPath {
    /// 游戏 ID，仅支持 ys、sr、zzz
    game_id: String,
    /// 数据集合 ID
    collection: String,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub(super) struct GameDataDetailPath {
    /// 游戏 ID，仅支持 ys、sr、zzz
    game_id: String,
    /// 数据集合 ID
    collection: String,
    /// 数据条目 ID
    id: String,
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/data",
    tag = "Game Data",
    summary = "获取游戏数据集合",
    description = "列出后端已保存的游戏数据集合和条目数量",
    params(crate::http::path::GamePath),
    responses(
        (status = 200, body = ListResponse<GameDataCollectionResponse>),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn collections(
    Path(crate::http::path::GamePath { game_id }): Path<crate::http::path::GamePath>,
    State(state): State<AppState>,
) -> Result<Json<ListResponse<GameDataCollectionResponse>>, AppError> {
    validate_game(&game_id)?;
    let items: Vec<GameDataCollectionResponse> = state
        .application()
        .list_game_data_collections(&game_id)
        .await?
        .into_iter()
        .map(GameDataCollectionResponse::from)
        .collect();
    Ok(Json(ListResponse {
        total: items.len() as u64,
        items,
    }))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/data/{collection}",
    tag = "Game Data",
    summary = "获取游戏数据列表",
    description = "分页读取一个游戏数据集合，返回摘要和后端资源链接；character 集合支持参数中注明的各游戏角色字段筛选",
    params(GameDataCollectionPath, GameDataListQuery),
    responses(
        (status = 200, body = PageResponse<GameDataEntryResponse>),
        (status = 400, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn list(
    Path(GameDataCollectionPath {
        game_id,
        collection,
    }): Path<GameDataCollectionPath>,
    State(state): State<AppState>,
    Query(query): Query<GameDataListQuery>,
) -> Result<Json<PageResponse<GameDataEntryResponse>>, AppError> {
    validate_game(&game_id)?;
    validate_collection(&collection)?;
    let filter = query.into_filter(game_id, collection)?;
    let limit = filter.limit;
    let offset = filter.offset;
    let (total, items) = state.application().list_game_data(filter).await?;
    let asset_base_url = &state.config().asset_base_url;
    Ok(Json(PageResponse {
        total,
        limit,
        offset,
        items: items
            .into_iter()
            .map(|item| GameDataEntryResponse::from_entry(item, asset_base_url))
            .collect(),
        meta: (),
    }))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/data/{collection}/{id}",
    tag = "Game Data",
    summary = "获取游戏数据详情",
    description = "读取一个游戏数据条目的完整摘要、详情和后端资源链接",
    params(GameDataDetailPath),
    responses(
        (status = 200, body = GameDataDetailResponse),
        (status = 400, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn detail(
    Path(GameDataDetailPath {
        game_id,
        collection,
        id,
    }): Path<GameDataDetailPath>,
    State(state): State<AppState>,
) -> Result<Json<GameDataDetailResponse>, AppError> {
    validate_game(&game_id)?;
    validate_collection(&collection)?;
    let item = state
        .application()
        .find_game_data(&game_id, &collection, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("game data entry not found".to_owned()))?;
    Ok(Json(GameDataDetailResponse::from_entry(
        item,
        &state.config().asset_base_url,
    )))
}

pub(super) fn validate_game(game_id: &str) -> Result<(), AppError> {
    if matches!(game_id, "ys" | "sr" | "zzz") {
        Ok(())
    } else {
        Err(AppError::NotFound(format!(
            "game data is not available for game {game_id}"
        )))
    }
}

pub(super) fn validate_collection(collection: &str) -> Result<(), AppError> {
    if !collection.is_empty()
        && collection.len() <= 64
        && collection
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'_' | b'-'))
    {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "invalid game data collection".to_owned(),
        ))
    }
}
