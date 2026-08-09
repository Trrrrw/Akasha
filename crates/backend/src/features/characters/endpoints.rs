use axum::{
    Json,
    extract::{Path, Query, State},
};

use crate::{
    features::characters::{dto::CharacterResponse, query::CharacterListQuery},
    http::{
        error::AppError,
        path::GamePath,
        response::{ErrorResponse, PageResponse},
    },
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/games/{game_id}/characters",
    tag = "Character",
    summary = "获取角色列表",
    description = "按名称、性别、配音演员和生日月份筛选角色，并返回分页结果",
    params(GamePath, CharacterListQuery),
    responses(
        (status = 200, body = PageResponse<CharacterResponse>),
        (status = 400, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
/// 按请求的筛选条件和分页窗口列出游戏角色
pub(super) async fn list(
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
    Query(query): Query<CharacterListQuery>,
) -> Result<Json<PageResponse<CharacterResponse>>, AppError> {
    let filter = query.into_filter(game_id)?;
    let limit = filter.limit;
    let offset = filter.offset;
    let (total, rows) = state.application().list_characters(filter).await?;

    Ok(Json(PageResponse {
        total,
        limit,
        offset,
        items: rows.into_iter().map(CharacterResponse::from).collect(),
        meta: (),
    }))
}
