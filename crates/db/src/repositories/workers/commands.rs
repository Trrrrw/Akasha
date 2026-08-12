use crate::{
    Db, DbError,
    entities::worker_states,
    models::{WorkerPhase, WorkerStatus},
};
use akasha_application::workers::{
    WorkerAcquireRequest, WorkerAcquireResult, WorkerCompleteCommand,
    WorkerPhase as ApplicationWorkerPhase, WorkerUpdateCheckpointCommand,
};
use chrono::{DateTime, Duration, FixedOffset, Utc};
use sea_orm::{ActiveValue::Set, ColumnTrait, Condition, EntityTrait, QueryFilter};
use serde_json::json;

use super::projections::WorkerState;

const LEASE_DURATION: Duration = Duration::minutes(2);

/// 在不存在有效竞争租约时获取 worker 租约
pub async fn acquire_worker(
    db: &Db,
    request: WorkerAcquireRequest,
) -> Result<WorkerAcquireResult, DbError> {
    let now = Utc::now().fixed_offset();

    // 已存在的 worker 直接进入租约更新，避免每次获取都创建一次冲突插入版本
    ensure_worker_state(db, &request, now).await?;

    let run_id = request.run_id;
    let lease_until = now + LEASE_DURATION;
    let update = worker_states::Entity::update_many()
        .set(worker_states::ActiveModel {
            status: Set(WorkerStatus::Running),
            run_id: Set(Some(run_id.clone())),
            lease_until: Set(Some(lease_until)),
            last_error: Set(None),
            updated_at: Set(now),
            ..Default::default()
        })
        .filter(worker_states::Column::Id.eq(&request.worker_id))
        .filter(
            Condition::any()
                .add(worker_states::Column::Status.ne(WorkerStatus::Running))
                .add(worker_states::Column::LeaseUntil.is_null())
                .add(worker_states::Column::LeaseUntil.lt(now))
                .add(worker_states::Column::LeaseUntil.gt(now + LEASE_DURATION))
                .add(worker_states::Column::RunId.eq(run_id)),
        )
        .exec(db.conn())
        .await
        .map_err(DbError::Query)?;

    let state = find_by_id(db, &request.worker_id).await?;
    if update.rows_affected == 1 {
        Ok(WorkerAcquireResult::Acquired(state))
    } else {
        Ok(WorkerAcquireResult::Busy(state))
    }
}

/// 确保 worker 状态存在，仅在首次创建或并发创建时执行冲突插入
async fn ensure_worker_state(
    db: &Db,
    request: &WorkerAcquireRequest,
    now: DateTime<FixedOffset>,
) -> Result<(), DbError> {
    let state_exists = worker_states::Entity::find_by_id(&request.worker_id)
        .one(db.conn())
        .await
        .map_err(DbError::Query)?
        .is_some();

    if state_exists {
        return Ok(());
    }

    // 保留唯一键冲突保护，处理两个进程同时首次创建同一 worker 的竞态
    worker_states::Entity::insert(worker_states::ActiveModel {
        id: Set(request.worker_id.clone()),
        worker_type: Set(request.worker_type.clone()),
        source_id: Set(request.source_id.clone()),
        game_id: Set(request.game_id.clone()),
        phase: Set(WorkerPhase::InitialBackfill),
        status: Set(WorkerStatus::Idle),
        checkpoint: Set(json!({})),
        run_id: Set(None),
        lease_until: Set(None),
        last_error: Set(None),
        last_success_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    })
    .on_conflict_do_nothing()
    .exec(db.conn())
    .await
    .map_err(DbError::Query)?;

    Ok(())
}

/// 保存检查点并续期匹配的 worker 租约
pub async fn checkpoint_worker(
    db: &Db,
    command: WorkerUpdateCheckpointCommand,
) -> Result<bool, DbError> {
    let now = Utc::now().fixed_offset();
    let result = worker_states::Entity::update_many()
        .set(worker_states::ActiveModel {
            checkpoint: Set(command.checkpoint),
            lease_until: Set(Some(now + LEASE_DURATION)),
            updated_at: Set(now),
            ..Default::default()
        })
        .filter(worker_states::Column::Id.eq(command.worker_id))
        .filter(worker_states::Column::RunId.eq(command.run_id))
        .filter(worker_states::Column::Status.eq(WorkerStatus::Running))
        .exec(db.conn())
        .await
        .map_err(DbError::Query)?;

    Ok(result.rows_affected == 1)
}

/// 续期匹配 worker run 的租约
pub async fn heartbeat_worker(db: &Db, worker_id: String, run_id: String) -> Result<bool, DbError> {
    let now = Utc::now().fixed_offset();
    let result = worker_states::Entity::update_many()
        .set(worker_states::ActiveModel {
            lease_until: Set(Some(now + LEASE_DURATION)),
            updated_at: Set(now),
            ..Default::default()
        })
        .filter(worker_states::Column::Id.eq(worker_id))
        .filter(worker_states::Column::RunId.eq(run_id))
        .filter(worker_states::Column::Status.eq(WorkerStatus::Running))
        .exec(db.conn())
        .await
        .map_err(DbError::Query)?;

    Ok(result.rows_affected == 1)
}

/// 完成匹配的 worker run 并记录最终检查点
pub async fn complete_worker(db: &Db, command: WorkerCompleteCommand) -> Result<bool, DbError> {
    let now = Utc::now().fixed_offset();
    let result = worker_states::Entity::update_many()
        .set(worker_states::ActiveModel {
            phase: Set(worker_phase_record(command.phase)),
            status: Set(WorkerStatus::Idle),
            checkpoint: Set(command.checkpoint),
            lease_until: Set(None),
            last_error: Set(None),
            last_success_at: Set(Some(now)),
            updated_at: Set(now),
            ..Default::default()
        })
        .filter(worker_states::Column::Id.eq(command.worker_id))
        .filter(worker_states::Column::RunId.eq(command.run_id))
        .filter(
            Condition::any()
                .add(worker_states::Column::Status.eq(WorkerStatus::Running))
                .add(worker_states::Column::Status.eq(WorkerStatus::Idle)),
        )
        .exec(db.conn())
        .await
        .map_err(DbError::Query)?;

    Ok(result.rows_affected == 1)
}

/// 将匹配的 worker run 标记为失败
pub async fn fail_worker(
    db: &Db,
    worker_id: String,
    run_id: String,
    error_message: String,
) -> Result<bool, DbError> {
    let now = Utc::now().fixed_offset();
    let result = worker_states::Entity::update_many()
        .set(worker_states::ActiveModel {
            status: Set(WorkerStatus::Failed),
            lease_until: Set(None),
            last_error: Set(Some(error_message)),
            updated_at: Set(now),
            ..Default::default()
        })
        .filter(worker_states::Column::Id.eq(worker_id))
        .filter(worker_states::Column::RunId.eq(run_id))
        .filter(
            Condition::any()
                .add(worker_states::Column::Status.eq(WorkerStatus::Running))
                .add(worker_states::Column::Status.eq(WorkerStatus::Failed)),
        )
        .exec(db.conn())
        .await
        .map_err(DbError::Query)?;

    Ok(result.rows_affected == 1)
}

/// 在条件租约更新后加载一个 worker 状态
async fn find_by_id(db: &Db, worker_id: &str) -> Result<WorkerState, DbError> {
    worker_states::Entity::find_by_id(worker_id)
        .one(db.conn())
        .await
        .map_err(DbError::Query)?
        .map(WorkerState::from)
        .ok_or_else(|| DbError::Query(sea_orm::DbErr::RecordNotFound(worker_id.to_owned())))
}

/// 将应用层 worker 阶段转换为 SeaORM 存储的枚举
fn worker_phase_record(phase: ApplicationWorkerPhase) -> WorkerPhase {
    match phase {
        ApplicationWorkerPhase::InitialBackfill => WorkerPhase::InitialBackfill,
        ApplicationWorkerPhase::Incremental => WorkerPhase::Incremental,
    }
}
