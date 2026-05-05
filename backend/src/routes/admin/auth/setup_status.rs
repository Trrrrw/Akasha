use axum::{Json, http::StatusCode};
use sea_orm::EntityTrait;
use serde::Serialize;

use db::entities::admin_users;

pub async fn get() -> Result<Json<SetupStatusResponse>, StatusCode> {
    let initialized = admin_users::Entity::find()
        .one(db::pool())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    Ok(Json(SetupStatusResponse { initialized }))
}

#[derive(Serialize)]
pub struct SetupStatusResponse {
    pub initialized: bool,
}
