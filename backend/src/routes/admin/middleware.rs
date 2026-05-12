use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use db::entities::admin_sessions;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use super::admin_token;

pub async fn admin_auth(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    let Some(token) = token else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let claims = admin_token::verify_access_token(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let now = Utc::now().fixed_offset();
    let session = admin_sessions::Entity::find()
        .filter(admin_sessions::Column::AccessTokenJti.eq(&claims.jti))
        .one(db::pool())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let Some(session) = session else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if session.revoked_at.is_some() || session.expires_at <= now {
        return Err(StatusCode::UNAUTHORIZED);
    }

    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
