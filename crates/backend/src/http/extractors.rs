use akasha_application::audit::{AuditActorType, AuditContext};
use axum::{extract::ConnectInfo, http::HeaderMap};
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;

use crate::{features::auth::token, http::error::AppError, state::AppState};
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};

/// 通过 bearer token 认证的数据写入主体
pub(crate) enum DataWriteActor {
    Admin {
        user_id: String,
        ip_address: Option<String>,
    },
    Worker {
        ip_address: Option<String>,
    },
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
        match self {
            Self::Admin { user_id, .. } => user_id,
            Self::Worker { .. } => "worker",
        }
    }

    /// 返回审计日志使用的主体类型和用户标识
    pub(crate) fn audit_identity(
        &self,
        worker_id: Option<String>,
    ) -> (AuditActorType, Option<String>) {
        match self {
            Self::Admin { user_id, .. } => (AuditActorType::User, Some(user_id.clone())),
            Self::Worker { .. } => (AuditActorType::Worker, worker_id),
        }
    }

    /// 返回连接层识别到的请求来源地址
    fn ip_address(&self) -> Option<String> {
        match self {
            Self::Admin { ip_address, .. } | Self::Worker { ip_address } => ip_address.clone(),
        }
    }

    /// 根据认证主体、请求头和请求附加信息构造审计上下文
    pub(crate) fn audit_context(&self, request: AuditRequest, headers: &HeaderMap) -> AuditContext {
        let (actor_type, actor_id) = self.audit_identity(request.worker_id.clone());
        AuditContext {
            actor_type,
            actor_id,
            operation: request.operation.unwrap_or_else(|| "sync".to_owned()),
            request_id: headers
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
            ip_address: self.ip_address(),
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
    /// 从 bearer token 提取经过授权的数据写入主体
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
        let ip_address = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(address)| address.ip().to_string());
        if token::sensitive_tokens_match(
            &state.config().auth,
            bearer_token,
            &state.config().worker.token,
        )? {
            return Ok(Self::Worker { ip_address });
        }
        let user_id = token::verify_access_token(&state.config().auth, bearer_token)?
            .sub
            .parse()
            .map_err(|_| AppError::Unauthorized("invalid access token subject".into()))?;
        let user = state
            .application()
            .find_current_user(user_id)
            .await?
            .ok_or_else(|| AppError::Unauthorized("invalid user".into()))?;
        if !user.is_admin {
            return Err(AppError::Forbidden("admin permission required".into()));
        }
        Ok(Self::Admin {
            user_id: user.id.to_string(),
            ip_address,
        })
    }
}
