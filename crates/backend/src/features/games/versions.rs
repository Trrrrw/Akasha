use akasha_application::game_versions::GameVersion;
use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    http::{
        error::AppError,
        path::{GamePath, require_game},
        response::{ErrorResponse, ListResponse, utc_timestamp},
    },
    state::AppState,
};

/// 公开游戏版本时间线条目
#[derive(Serialize, ToSchema)]
#[schema(description = "游戏版本及其有效时间范围")]
pub(super) struct GameVersionResponse {
    /// 版本 ID
    id: String,
    /// 版本名称
    name: Option<String>,
    /// UTC RFC 3339 格式的版本开始时间
    start_time: String,
    /// UTC RFC 3339 格式的版本结束时间，最新版本可能为空
    end_time: Option<String>,
    /// 时间状态
    time_status: String,
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/versions",
    tag = "Game",
    summary = "获取游戏版本时间线",
    description = "按开始时间返回指定游戏的版本及其有效时间范围",
    params(GamePath),
    responses(
        (status = 200, body = ListResponse<GameVersionResponse>),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
/// 按开始时间返回指定游戏的公开版本时间线
pub(super) async fn list(
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
) -> Result<Json<ListResponse<GameVersionResponse>>, AppError> {
    require_game(&state, &game_id).await?;
    let items = state
        .application()
        .list_game_versions(&game_id)
        .await?
        .into_iter()
        .map(GameVersionResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(ListResponse {
        total: items.len() as u64,
        items,
    }))
}

impl From<GameVersion> for GameVersionResponse {
    fn from(value: GameVersion) -> Self {
        Self {
            id: value.id,
            name: value.name,
            start_time: utc_timestamp(value.start_time),
            end_time: value.end_time.map(utc_timestamp),
            time_status: value.time_status,
        }
    }
}
