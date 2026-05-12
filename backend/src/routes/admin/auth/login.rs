use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{Json, http::StatusCode};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use super::super::admin_token;
use db::entities::{admin_sessions, admin_users};

pub async fn post(
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<LoginErrorResponse>)> {
    let Some(user) = admin_users::Entity::find()
        .filter(admin_users::Column::UserName.eq(&req.username))
        .one(db::pool())
        .await
        .map_err(internal_error)?
    else {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "invalid username or password",
        ));
    };

    verify_password(&req.password, &user.password_hash)
        .map_err(|_| error(StatusCode::UNAUTHORIZED, "invalid username or password"))?;

    let refresh_jti = admin_token::new_token_jti();
    let now = Utc::now().fixed_offset();
    let refresh_expires_at =
        (Utc::now() + Duration::seconds(admin_token::REFRESH_EXPIRES_IN)).fixed_offset();
    let tokens = admin_token::issue_admin_tokens(&user.user_name, &refresh_jti)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "failed to issue token"))?;

    admin_sessions::ActiveModel {
        admin_user_id: Set(user.id),
        access_token_jti: Set(Some(tokens.access_jti)),
        refresh_token_jti: Set(refresh_jti.clone()),
        user_agent: Set(None),
        ip_address: Set(None),
        expires_at: Set(refresh_expires_at),
        revoked_at: Set(None),
        last_used_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db::pool())
    .await
    .map_err(internal_error)?;

    Ok(Json(LoginResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
        user: LoginUser {
            id: user.id,
            username: user.user_name,
        },
    }))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: LoginUser,
}

#[derive(Serialize)]
pub struct LoginUser {
    pub id: i32,
    pub username: String,
}

#[derive(Serialize)]
pub struct LoginErrorResponse {
    pub message: String,
}

fn verify_password(
    password: &str,
    password_hash: &str,
) -> Result<(), argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(password_hash)?;
    Argon2::default().verify_password(password.as_bytes(), &parsed_hash)
}

fn internal_error(err: sea_orm::DbErr) -> (StatusCode, Json<LoginErrorResponse>) {
    error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn error(status: StatusCode, message: impl Into<String>) -> (StatusCode, Json<LoginErrorResponse>) {
    (
        status,
        Json(LoginErrorResponse {
            message: message.into(),
        }),
    )
}
