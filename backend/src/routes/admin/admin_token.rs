use std::{env, sync::OnceLock};

use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};

pub const ACCESS_EXPIRES_IN: i64 = 15 * 60;
pub const REFRESH_EXPIRES_IN: i64 = 7 * 24 * 60 * 60;

static JWT_SECRET: OnceLock<String> = OnceLock::new();

pub struct AdminTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminClaims {
    pub sub: String,
    pub username: String,
    pub token_type: AdminTokenType,
    pub jti: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminTokenType {
    Access,
    Refresh,
}

pub fn new_token_jti() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 48)
}

pub fn issue_admin_tokens(
    username: &str,
    refresh_jti: &str,
) -> Result<AdminTokens, jsonwebtoken::errors::Error> {
    let access_jti = new_token_jti();
    let access_token = issue_token(
        username,
        AdminTokenType::Access,
        &access_jti,
        ACCESS_EXPIRES_IN,
    )?;
    let refresh_token = issue_token(
        username,
        AdminTokenType::Refresh,
        refresh_jti,
        REFRESH_EXPIRES_IN,
    )?;

    Ok(AdminTokens {
        access_token,
        refresh_token,
        expires_in: ACCESS_EXPIRES_IN,
    })
}

pub fn verify_access_token(token: &str) -> Result<AdminClaims, jsonwebtoken::errors::Error> {
    let claims = verify_token(token)?;

    if claims.token_type != AdminTokenType::Access {
        return Err(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        ));
    }

    Ok(claims)
}

pub fn verify_refresh_token(token: &str) -> Result<AdminClaims, jsonwebtoken::errors::Error> {
    let claims = verify_token(token)?;

    if claims.token_type != AdminTokenType::Refresh {
        return Err(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        ));
    }

    Ok(claims)
}

fn issue_token(
    username: &str,
    token_type: AdminTokenType,
    jti: &str,
    expires_in: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = AdminClaims {
        sub: username.to_string(),
        username: username.to_string(),
        token_type,
        jti: jti.to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::seconds(expires_in)).timestamp(),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
}

fn verify_token(token: &str) -> Result<AdminClaims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let token = decode::<AdminClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &validation,
    )?;

    Ok(token.claims)
}

fn jwt_secret() -> &'static str {
    JWT_SECRET
        .get_or_init(|| env::var("ADMIN_JWT_SECRET").expect("缺少必需的环境变量 ADMIN_JWT_SECRET"))
        .as_str()
}
