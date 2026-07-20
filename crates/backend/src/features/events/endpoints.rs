use axum::extract::Path;

use crate::http::{error::AppError, path::GamePath};

#[utoipa::path(
    get,
    path = "/games/{game_id}/events",
    tag = "Event",
    summary = "获取活动列表",
    params(GamePath),
    responses((status = 501, body = crate::http::response::ErrorResponse))
)]
pub(super) async fn list(Path(GamePath { game_id: _ }): Path<GamePath>) -> Result<(), AppError> {
    Err(AppError::NotImplemented("接口尚未实现".to_string()))
}
