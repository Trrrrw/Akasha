use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use crate::{ApplicationError, ApplicationRepository, ApplicationServices};

/// 审计日志中的操作主体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditActorType {
    User,
    Worker,
    System,
}

/// 一次写入操作使用的审计上下文
#[derive(Debug, Clone)]
pub struct AuditContext {
    pub actor_type: AuditActorType,
    pub actor_id: Option<String>,
    pub operation: String,
    pub request_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Value,
}

impl<R> ApplicationServices<R>
where
    R: ApplicationRepository,
{
    /// 删除超过保留期限的审计日志
    pub async fn delete_audit_logs_before(
        &self,
        cutoff: DateTime<FixedOffset>,
    ) -> Result<u64, ApplicationError> {
        Ok(self.repository.delete_audit_logs_before(cutoff).await?)
    }
}
