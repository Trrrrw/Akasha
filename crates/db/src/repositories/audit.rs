use akasha_application::audit::{AuditActorType, AuditContext};
use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseTransaction, DbErr, EntityTrait, QueryFilter,
};
use serde_json::{Map, Value, json};

use crate::{Db, entities::audit_logs, models};

/// 删除创建时间早于截止时间的审计日志
pub(crate) async fn delete_before(db: &Db, cutoff: DateTime<FixedOffset>) -> Result<u64, DbErr> {
    let result = audit_logs::Entity::delete_many()
        .filter(audit_logs::Column::CreatedAt.lt(cutoff))
        .exec(db.conn())
        .await?;

    Ok(result.rows_affected)
}

/// 在当前事务中写入一条审计日志
pub(crate) async fn insert(
    txn: &DatabaseTransaction,
    context: &AuditContext,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<String>,
    extra_metadata: Value,
) -> Result<(), DbErr> {
    let mut metadata = match context.metadata.clone() {
        Value::Object(value) => value,
        _ => Map::new(),
    };
    metadata.insert("operation".to_owned(), json!(context.operation));
    if let Value::Object(extra) = extra_metadata {
        metadata.extend(extra);
    }

    audit_logs::Entity::insert(audit_logs::ActiveModel {
        actor_type: Set(actor_type(context.actor_type)),
        actor_id: Set(context.actor_id.clone()),
        action: Set(action.to_owned()),
        target_type: Set(target_type.map(ToOwned::to_owned)),
        target_id: Set(target_id),
        request_id: Set(context.request_id.clone()),
        ip_address: Set(context.ip_address.clone()),
        user_agent: Set(context.user_agent.clone()),
        metadata: Set(Some(Value::Object(metadata))),
        created_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    })
    .exec(txn)
    .await
    .map(|_| ())
}

/// 将应用层审计主体转换为数据库枚举
fn actor_type(value: AuditActorType) -> models::AuditLogActorType {
    match value {
        AuditActorType::User => models::AuditLogActorType::User,
        AuditActorType::Worker => models::AuditLogActorType::Worker,
        AuditActorType::System => models::AuditLogActorType::System,
    }
}
