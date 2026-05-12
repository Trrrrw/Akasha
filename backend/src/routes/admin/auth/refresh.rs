use axum::{Json, http::StatusCode};
use chrono::Utc;
use db::entities::admin_sessions;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use super::super::admin_token;

pub async fn post(
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, (StatusCode, Json<RefreshErrorResponse>)> {
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

    if session.revoked_at.is_some() || session.expires_at <= now {
        return Err(error(StatusCode::UNAUTHORIZED, "invalid refresh token"));
    }

    let new_refresh_jti = admin_token::new_token_jti();
    let tokens = admin_token::issue_admin_tokens(&claims.username, &new_refresh_jti)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "failed to issue token"))?;

    let mut active_session: admin_sessions::ActiveModel = session.into();
    active_session.access_token_jti = Set(Some(tokens.access_jti));
    active_session.refresh_token_jti = Set(new_refresh_jti.clone());
    active_session.last_used_at = Set(Some(now));
    active_session.updated_at = Set(now);

    active_session
        .update(db::pool())
        .await
        .map_err(internal_error)?;

    Ok(Json(RefreshResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
    }))
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Serialize)]
pub struct RefreshErrorResponse {
    pub message: String,
}

fn internal_error(err: sea_orm::DbErr) -> (StatusCode, Json<RefreshErrorResponse>) {
    error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn error(
    status: StatusCode,
    message: impl Into<String>,
) -> (StatusCode, Json<RefreshErrorResponse>) {
    (
        status,
        Json(RefreshErrorResponse {
            message: message.into(),
        }),
    )
}
