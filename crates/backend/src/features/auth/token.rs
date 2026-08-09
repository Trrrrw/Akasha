use akasha_application::auth::AuthUser;
use axum::http::{HeaderMap, header};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{config::AuthConfig, http::error::AppError};

/// OAuth state Cookie 名称
pub(crate) const OAUTH_STATE_COOKIE: &str = "akasha_oauth_state";
/// refresh token Cookie 名称
pub(crate) const REFRESH_TOKEN_COOKIE: &str = "akasha_refresh_token";
/// access token 的固定有效期，单位为秒
pub(crate) const ACCESS_TOKEN_TTL_SECONDS: u64 = 15 * 60;

/// 签名 access token 中保存的声明
#[derive(Serialize, Deserialize)]
pub(crate) struct AccessTokenClaims {
    /// 用户 ID
    pub sub: String,
    typ: String,
    iat: i64,
    exp: i64,
}

/// 为 OAuth state 和 token 材料创建随机 URL 安全值
pub(crate) fn generate_random_token() -> String {
    let mut bytes = [0; 32];
    rand::fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// 创建带有稳定应用前缀的 refresh token
pub(crate) fn generate_refresh_token() -> String {
    format!("ak_rt_{}", generate_random_token())
}

/// 计算用于存储敏感 token 值的带密钥哈希
pub(crate) fn hash_sensitive_token(config: &AuthConfig, value: &str) -> Result<String, AppError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(config.token_hash_secret.as_bytes())
        .map_err(|error| AppError::Internal(error.into()))?;
    mac.update(value.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// 以固定时间比较两个敏感 token 值
pub(crate) fn sensitive_tokens_match(
    config: &AuthConfig,
    provided: &str,
    expected: &str,
) -> Result<bool, AppError> {
    let mut expected_mac = Hmac::<Sha256>::new_from_slice(config.token_hash_secret.as_bytes())
        .map_err(|error| AppError::Internal(error.into()))?;
    expected_mac.update(expected.as_bytes());
    let expected_tag = expected_mac.finalize().into_bytes();

    let mut provided_mac = Hmac::<Sha256>::new_from_slice(config.token_hash_secret.as_bytes())
        .map_err(|error| AppError::Internal(error.into()))?;
    provided_mac.update(provided.as_bytes());

    Ok(provided_mac.verify_slice(&expected_tag).is_ok())
}

/// 为已认证用户签发带签名的短期 bearer token
pub(crate) fn issue_access_token(config: &AuthConfig, user: &AuthUser) -> Result<String, AppError> {
    let now = Utc::now().timestamp();
    encode(
        &Header::new(Algorithm::HS256),
        &AccessTokenClaims {
            sub: user.id.to_string(),
            typ: "access".into(),
            iat: now,
            exp: now + ACCESS_TOKEN_TTL_SECONDS as i64,
        },
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|error| AppError::Internal(error.into()))
}

/// 验证 bearer token 并确保其类型为 access token
pub(crate) fn verify_access_token(
    config: &AuthConfig,
    value: &str,
) -> Result<AccessTokenClaims, AppError> {
    let claims = decode::<AccessTokenClaims>(
        value,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| AppError::Unauthorized("invalid access token".into()))?
    .claims;
    if claims.typ != "access" {
        return Err(AppError::Unauthorized("invalid token type".into()));
    }
    Ok(claims)
}

/// 从 HTTP Cookie 响应头读取指定名称的 Cookie 值
pub(crate) fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(key, value)| (key == name).then(|| value.to_string()))
}

/// 验证回调 OAuth state 与 state Cookie 相匹配
pub(crate) fn validate_state(
    config: &AuthConfig,
    headers: &HeaderMap,
    state: &str,
) -> Result<(), AppError> {
    let cookie_state = read_cookie(headers, OAUTH_STATE_COOKIE)
        .ok_or_else(|| AppError::BadRequest("invalid oauth state".into()))?;
    if sensitive_tokens_match(config, state, &cookie_state)? {
        Ok(())
    } else {
        Err(AppError::BadRequest("invalid oauth state".into()))
    }
}

#[cfg(test)]
mod tests {
    use akasha_application::auth::AuthUser;
    use uuid::Uuid;

    use crate::{config::AuthConfig, http::error::AppError};

    use super::{issue_access_token, sensitive_tokens_match, verify_access_token};

    /// 创建认证单元测试使用的固定密钥
    fn auth_config() -> AuthConfig {
        AuthConfig {
            jwt_secret: "j".repeat(32),
            token_hash_secret: "h".repeat(32),
        }
    }

    /// 签发的 access token 可以按固定算法验证
    #[test]
    fn issues_and_verifies_access_tokens() {
        let config = auth_config();
        let user_id = Uuid::new_v4();
        let encoded = issue_access_token(
            &config,
            &AuthUser {
                id: user_id,
                display_name: "测试用户".to_owned(),
                avatar_url: None,
                is_admin: false,
            },
        )
        .expect("应成功签发 access token");

        let claims = verify_access_token(&config, &encoded).expect("应成功验证 access token");

        assert_eq!(claims.sub, user_id.to_string());
    }

    /// 敏感 token 比较只接受完全相同的值
    #[test]
    fn compares_sensitive_tokens() {
        let config = auth_config();

        assert!(sensitive_tokens_match(&config, "same", "same").expect("应完成 token 比较"));
        assert!(!sensitive_tokens_match(&config, "left", "right").expect("应完成 token 比较"));
    }

    /// 无效 access token 返回未认证错误
    #[test]
    fn rejects_invalid_access_tokens_as_unauthorized() {
        let result = verify_access_token(&auth_config(), "invalid");

        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }
}
