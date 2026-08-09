use axum::extract::Path;

use crate::http::{error::AppError, path::GamePath};

#[utoipa::path(
    get,
    path = "/games/{game_id}/calendar",
    tag = "Calendar",
    summary = "获取日历",
    description = "尚未实现，调用后固定返回 501",
    params(GamePath),
    responses((status = 501, body = crate::http::response::ErrorResponse))
)]
/// 游戏日历接口的占位实现
pub(super) async fn list(Path(GamePath { game_id: _ }): Path<GamePath>) -> Result<(), AppError> {
    Err(AppError::NotImplemented("接口尚未实现".to_string()))
}

#[utoipa::path(
    get,
    path = "/games/{game_id}/calendar/ics",
    tag = "Calendar",
    summary = "获取日历 ICS",
    description = "尚未实现，调用后固定返回 501",
    params(GamePath),
    responses((status = 501, body = crate::http::response::ErrorResponse))
)]
/// iCalendar 导出接口的占位实现
pub(super) async fn ics(Path(GamePath { game_id: _ }): Path<GamePath>) -> Result<(), AppError> {
    Err(AppError::NotImplemented("接口尚未实现".to_string()))
}
