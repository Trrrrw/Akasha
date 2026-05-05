use axum::{
    Json,
    http::{HeaderMap, StatusCode},
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde::{Deserialize, Serialize};

use crate::setup_token;
use db::entities::admin_users;

pub async fn post(
    headers: HeaderMap,
    Json(req): Json<SetupRequest>,
) -> Result<Json<SetupResponse>, (StatusCode, Json<SetupErrorResponse>)> {
    setup_token::verify_setup_token(&headers)
        .map_err(|status| error(status, "invalid setup token"))?;

    let initialized = admin_users::Entity::find()
        .one(db::pool())
        .await
        .map_err(internal_error)?
        .is_some();

    if initialized {
        return Err(error(
            StatusCode::CONFLICT,
            "admin user already initialized",
        ));
    }

    let password_hash = hash_password(&req.password)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "failed to hash password"))?;

    let now = chrono::Utc::now().fixed_offset();

    admin_users::ActiveModel {
        user_name: Set(req.username),
        password_hash: Set(password_hash),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db::pool())
    .await
    .map_err(internal_error)?;

    Ok(Json(SetupResponse { success: true }))
}

#[derive(Deserialize)]
pub struct SetupRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct SetupResponse {
    pub success: bool,
}

#[derive(Serialize)]
pub struct SetupErrorResponse {
    pub message: String,
}

fn error(status: StatusCode, message: impl Into<String>) -> (StatusCode, Json<SetupErrorResponse>) {
    (
        status,
        Json(SetupErrorResponse {
            message: message.into(),
        }),
    )
}

fn internal_error(err: sea_orm::DbErr) -> (StatusCode, Json<SetupErrorResponse>) {
    error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
    };

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;

    Ok(hash.to_string())
}
