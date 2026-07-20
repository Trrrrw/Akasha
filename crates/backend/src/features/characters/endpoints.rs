use axum::{
    Json,
    extract::{Path, Query, State},
};

use crate::{
    features::characters::{dto::CharacterResponse, query::CharacterListQuery, use_cases},
    http::{
        error::AppError,
        path::GamePath,
        response::{ErrorResponse, PageResponse},
    },
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/games/{game_id}/chars",
    tag = "Character",
    summary = "获取角色列表",
    params(GamePath, CharacterListQuery),
    responses(
        (status = 200, body = PageResponse<CharacterResponse>),
        (status = 500, body = ErrorResponse)
    )
)]
pub(super) async fn list(
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
    Query(query): Query<CharacterListQuery>,
) -> Result<Json<PageResponse<CharacterResponse>>, AppError> {
    let filter = query.into_filter(game_id);
    let limit = filter.limit;
    let offset = filter.offset;
    let (total, rows) = use_cases::list(state.db(), filter)
        .await
        .map_err(|error| AppError::Internal(error.into()))?;

    Ok(Json(PageResponse {
        total,
        limit,
        offset,
        items: rows.into_iter().map(CharacterResponse::from).collect(),
        meta: (),
    }))
}
