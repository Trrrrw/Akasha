use crate::{
    Db, DbError,
    entities::worker_states,
    models::{WorkerPhase, WorkerStatus},
};
use chrono::{Duration, Utc};
use sea_orm::{ActiveValue::Set, ColumnTrait, Condition, EntityTrait, QueryFilter};
use serde_json::{Value, json};

use super::projections::WorkerState;

const LEASE_DURATION: Duration = Duration::minutes(2);

pub struct AcquireWorkerInput {
    pub id: String,
    pub run_id: String,
    pub worker_type: String,
    pub source_id: Option<String>,
    pub game_id: String,
}

pub enum AcquireWorkerResult {
    Acquired(WorkerState),
    Busy(WorkerState),
}

pub struct UpdateCheckpointInput {
    pub id: String,
    pub run_id: String,
    pub checkpoint: Value,
}

pub struct CompleteWorkerInput {
    pub id: String,
    pub run_id: String,
    pub phase: WorkerPhase,
    pub checkpoint: Value,
}

pub async fn acquire(db: &Db, input: AcquireWorkerInput) -> Result<AcquireWorkerResult, DbError> {
    let now = Utc::now().fixed_offset();

    worker_states::Entity::insert(worker_states::ActiveModel {
        id: Set(input.id.clone()),
        worker_type: Set(input.worker_type),
        source_id: Set(input.source_id),
        game_id: Set(input.game_id),
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

    let run_id = input.run_id;
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
        .filter(worker_states::Column::Id.eq(&input.id))
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

    let state = find_by_id(db, &input.id).await?;
    if update.rows_affected == 1 {
        Ok(AcquireWorkerResult::Acquired(state))
    } else {
        Ok(AcquireWorkerResult::Busy(state))
    }
}

pub async fn checkpoint(db: &Db, input: UpdateCheckpointInput) -> Result<bool, DbError> {
    let now = Utc::now().fixed_offset();
    let result = worker_states::Entity::update_many()
        .set(worker_states::ActiveModel {
            checkpoint: Set(input.checkpoint),
            lease_until: Set(Some(now + LEASE_DURATION)),
            updated_at: Set(now),
            ..Default::default()
        })
        .filter(worker_states::Column::Id.eq(input.id))
        .filter(worker_states::Column::RunId.eq(input.run_id))
        .filter(worker_states::Column::Status.eq(WorkerStatus::Running))
        .exec(db.conn())
        .await
        .map_err(DbError::Query)?;

    Ok(result.rows_affected == 1)
}

pub async fn heartbeat(db: &Db, id: String, run_id: String) -> Result<bool, DbError> {
    let now = Utc::now().fixed_offset();
    let result = worker_states::Entity::update_many()
        .set(worker_states::ActiveModel {
            lease_until: Set(Some(now + LEASE_DURATION)),
            updated_at: Set(now),
            ..Default::default()
        })
        .filter(worker_states::Column::Id.eq(id))
        .filter(worker_states::Column::RunId.eq(run_id))
        .filter(worker_states::Column::Status.eq(WorkerStatus::Running))
        .exec(db.conn())
        .await
        .map_err(DbError::Query)?;

    Ok(result.rows_affected == 1)
}

pub async fn complete(db: &Db, input: CompleteWorkerInput) -> Result<bool, DbError> {
    let now = Utc::now().fixed_offset();
    let result = worker_states::Entity::update_many()
        .set(worker_states::ActiveModel {
            phase: Set(input.phase),
            status: Set(WorkerStatus::Idle),
            checkpoint: Set(input.checkpoint),
            lease_until: Set(None),
            last_error: Set(None),
            last_success_at: Set(Some(now)),
            updated_at: Set(now),
            ..Default::default()
        })
        .filter(worker_states::Column::Id.eq(input.id))
        .filter(worker_states::Column::RunId.eq(input.run_id))
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

pub async fn fail(db: &Db, id: String, run_id: String, error: String) -> Result<bool, DbError> {
    let now = Utc::now().fixed_offset();
    let result = worker_states::Entity::update_many()
        .set(worker_states::ActiveModel {
            status: Set(WorkerStatus::Failed),
            lease_until: Set(None),
            last_error: Set(Some(error)),
            updated_at: Set(now),
            ..Default::default()
        })
        .filter(worker_states::Column::Id.eq(id))
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

async fn find_by_id(db: &Db, id: &str) -> Result<WorkerState, DbError> {
    worker_states::Entity::find_by_id(id)
        .one(db.conn())
        .await
        .map_err(DbError::Query)?
        .map(WorkerState::from)
        .ok_or_else(|| DbError::Query(sea_orm::DbErr::RecordNotFound(id.to_owned())))
}
