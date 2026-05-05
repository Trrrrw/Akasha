use axum::{Extension, Json};
use serde::Serialize;

use super::super::admin_token::AdminClaims;

pub async fn get(Extension(claims): Extension<AdminClaims>) -> Json<MeResponse> {
    Json(MeResponse {
        username: claims.username,
    })
}

#[derive(Serialize)]
pub struct MeResponse {
    pub username: String,
}
