use axum::extract::Path;

use crate::http::{error::AppError, path::GamePath};

#[utoipa::path(
    get,
    path = "/games/{game_id}/calendar",
    tag = "Calendar",
    summary = "获取日历",
    params(GamePath),
    responses((status = 501, body = crate::http::response::ErrorResponse))
)]
pub(super) async fn list(Path(GamePath { game_id: _ }): Path<GamePath>) -> Result<(), AppError> {
    Err(AppError::NotImplemented("接口尚未实现".to_string()))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/calendar/ics",
    tag = "Calendar",
    summary = "获取日历 ICS",
    params(GamePath),
    responses((status = 501, body = crate::http::response::ErrorResponse))
)]
pub(super) async fn ics(Path(GamePath { game_id: _ }): Path<GamePath>) -> Result<(), AppError> {
    Err(AppError::NotImplemented("接口尚未实现".to_string()))
}
