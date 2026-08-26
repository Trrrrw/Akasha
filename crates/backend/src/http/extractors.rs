use akasha_application::audit::{AuditActorType, AuditContext};
use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{HeaderMap, header, request::Parts},
};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use serde_json::json;
use sha2::Sha256;
use std::net::SocketAddr;

use crate::{http::error::AppError, state::AppState};

/// 通过单一 bearer token 认证的数据写入主体
pub(crate) struct DataWriteActor {
    ip_address: Option<String>,
}

/// 管理写入请求附带的审计信息
#[derive(Default, Deserialize)]
pub(crate) struct AuditRequest {
    pub(crate) operation: Option<String>,
    pub(crate) fields: Option<Vec<String>>,
    pub(crate) worker_id: Option<String>,
    pub(crate) run_id: Option<String>,
}

impl DataWriteActor {
    /// 返回用于结构化日志的写入主体标签
    pub(crate) fn label(&self) -> &str {
        "data-writer"
    }

    /// 根据请求头和请求附加信息构造审计上下文
    pub(crate) fn audit_context(&self, request: AuditRequest, headers: &HeaderMap) -> AuditContext {
        AuditContext {
            actor_type: AuditActorType::Worker,
            actor_id: request
                .worker_id
                .clone()
                .or_else(|| Some("data-writer".to_owned())),
            operation: request.operation.unwrap_or_else(|| "sync".to_owned()),
            request_id: headers
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
            ip_address: self.ip_address.clone(),
            user_agent: headers
                .get("user-agent")
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
            metadata: json!({
                "fields": request.fields,
                "worker_id": request.worker_id,
                "run_id": request.run_id,
            }),
        }
    }
}

impl FromRequestParts<AppState> for DataWriteActor {
    type Rejection = AppError;

    /// 从 Authorization Bearer 头验证统一的数据写入凭据
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let bearer_token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::Unauthorized("missing bearer token".into()))?;

        if !tokens_match(bearer_token, &state.config().data_write_token)? {
            return Err(AppError::Unauthorized("invalid data write token".into()));
        }

        Ok(Self {
            ip_address: parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ConnectInfo(address)| address.ip().to_string()),
        })
    }
}

/// 通过固定长度 HMAC 标签比较敏感 token，避免直接字符串比较泄露时序信息
fn tokens_match(provided: &str, expected: &str) -> Result<bool, AppError> {
    const CONTEXT: &[u8] = b"akasha-data-write-token-v1";

    let mut expected_mac = Hmac::<Sha256>::new_from_slice(expected.as_bytes())
        .map_err(|error| AppError::Internal(error.into()))?;
    expected_mac.update(CONTEXT);
    let expected_tag = expected_mac.finalize().into_bytes();

    let mut provided_mac = Hmac::<Sha256>::new_from_slice(provided.as_bytes())
        .map_err(|error| AppError::Internal(error.into()))?;
    provided_mac.update(CONTEXT);

    Ok(provided_mac.verify_slice(&expected_tag).is_ok())
}

#[cfg(test)]
mod tests {
    use super::tokens_match;

    #[test]
    fn data_write_tokens_only_match_exact_values() {
        assert!(tokens_match("same", "same").expect("token comparison should succeed"));
        assert!(!tokens_match("left", "right").expect("token comparison should succeed"));
    }
}
