use axum::http::{HeaderMap, header};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{config::AuthConfig, http::error::AppError};

pub(crate) const OAUTH_STATE_COOKIE: &str = "akasha_oauth_state";
pub(crate) const REFRESH_TOKEN_COOKIE: &str = "akasha_refresh_token";

#[derive(Serialize, Deserialize)]
pub(crate) struct AccessTokenClaims {
    pub sub: String,
    typ: String,
    #[serde(rename = "is_admin")]
    _is_admin: bool,
    iat: i64,
    exp: i64,
}

pub(crate) fn random_token() -> String {
    let mut bytes = [0; 32];
    rand::fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
pub(crate) fn refresh_token() -> String {
    format!("ak_rt_{}", random_token())
}
pub(crate) fn hash(config: &AuthConfig, value: &str) -> Result<String, AppError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(config.token_hash_secret.as_bytes())
        .map_err(|e| AppError::Internal(e.into()))?;
    mac.update(value.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}
pub(crate) fn access_token(
    config: &AuthConfig,
    user: &akasha_db::repositories::auth::AuthUser,
) -> Result<String, AppError> {
    let now = Utc::now().timestamp();
    encode(
        &Header::default(),
        &AccessTokenClaims {
            sub: user.id.to_string(),
            typ: "access".into(),
            _is_admin: user.is_admin,
            iat: now,
            exp: now + 900,
        },
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(e.into()))
}
pub(crate) fn verify(config: &AuthConfig, value: &str) -> Result<AccessTokenClaims, AppError> {
    let claims = decode::<AccessTokenClaims>(
        value,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::BadRequest("invalid access token".into()))?
    .claims;
    if claims.typ != "access" {
        return Err(AppError::BadRequest("invalid token type".into()));
    }
    Ok(claims)
}
pub(crate) fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(key, value)| (key == name).then(|| value.to_string()))
}
pub(crate) fn validate_state(headers: &HeaderMap, state: &str) -> Result<(), AppError> {
    match cookie(headers, OAUTH_STATE_COOKIE).as_deref() {
        Some(value) if value == state => Ok(()),
        _ => Err(AppError::BadRequest("invalid oauth state".into())),
    }
}
