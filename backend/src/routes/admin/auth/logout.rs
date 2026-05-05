use axum::{Json, http::StatusCode};
use chrono::Utc;
use db::entities::admin_sessions;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use super::super::admin_token;

pub async fn post(
    Json(req): Json<LogoutRequest>,
) -> Result<Json<LogoutResponse>, (StatusCode, Json<LogoutErrorResponse>)> {
    let claims = admin_token::verify_refresh_token(&req.refresh_token)
        .map_err(|_| error(StatusCode::UNAUTHORIZED, "invalid refresh token"))?;

    let Some(session) = admin_sessions::Entity::find()
        .filter(admin_sessions::Column::RefreshTokenJti.eq(&claims.jti))
        .one(db::pool())
        .await
        .map_err(internal_error)?
    else {
        return Err(error(StatusCode::UNAUTHORIZED, "invalid refresh token"));
    };

    let now = Utc::now().fixed_offset();

    let mut active_session: admin_sessions::ActiveModel = session.into();
    active_session.revoked_at = Set(Some(now));
    active_session.updated_at = Set(now);

    active_session
        .update(db::pool())
        .await
        .map_err(internal_error)?;

    Ok(Json(LogoutResponse { success: true }))
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct LogoutResponse {
    pub success: bool,
}

#[derive(Serialize)]
pub struct LogoutErrorResponse {
    pub message: String,
}

fn internal_error(err: sea_orm::DbErr) -> (StatusCode, Json<LogoutErrorResponse>) {
    error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn error(
    status: StatusCode,
    message: impl Into<String>,
) -> (StatusCode, Json<LogoutErrorResponse>) {
    (
        status,
        Json(LogoutErrorResponse {
            message: message.into(),
        }),
    )
}
