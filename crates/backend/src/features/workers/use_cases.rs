use akasha_db::repositories::workers::{
    self, AcquireWorkerInput, AcquireWorkerResult, CompleteWorkerInput, UpdateCheckpointInput,
    WorkerPhase,
};
use serde_json::Value;

use crate::http::error::AppError;

use super::dto::{AcquireWorkerRequest, AcquireWorkerResponse, WorkerPhaseRequest};

const MAX_KEY_SEGMENT_LENGTH: usize = 64;
const MAX_ERROR_LENGTH: usize = 4_000;

pub(super) async fn acquire(
    db: &akasha_db::Db,
    request: AcquireWorkerRequest,
) -> Result<AcquireWorkerResponse, AppError> {
    let worker_type = normalize_key_segment("worker_type", request.worker_type)?;
    let game_id = normalize_key_segment("game_id", request.game_id)?;
    let acquire_id = normalize_run_id(request.acquire_id)?;
    let source_id = request
        .source_id
        .map(|value| normalize_key_segment("source_id", value))
        .transpose()?;

    if worker_type == "news" && source_id.is_none() {
        return Err(AppError::BadRequest(
            "source_id is required for news workers".into(),
        ));
    }

    let worker_id = build_worker_id(&worker_type, source_id.as_deref(), &game_id);
    let result = workers::acquire(
        db,
        AcquireWorkerInput {
            id: worker_id,
            run_id: acquire_id,
            worker_type,
            source_id,
            game_id,
        },
    )
    .await
    .map_err(|error| AppError::Internal(error.into()))?;

    match result {
        AcquireWorkerResult::Acquired(state) => {
            let run_id = state.run_id.ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("acquired worker has no run_id"))
            })?;
            let lease_until = state.lease_until.ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("acquired worker has no lease_until"))
            })?;

            Ok(AcquireWorkerResponse {
                worker_id: state.id,
                phase: state.phase.as_str().to_owned(),
                status: state.status.as_str().to_owned(),
                checkpoint: state.checkpoint,
                run_id,
                lease_until: lease_until.to_rfc3339(),
                last_success_at: state.last_success_at.map(|value| value.to_rfc3339()),
            })
        }
        AcquireWorkerResult::Busy(state) => {
            let lease = state
                .lease_until
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "unknown".into());
            Err(AppError::Conflict(format!(
                "worker {} is already running until {}",
                state.id, lease
            )))
        }
    }
}

pub(super) async fn checkpoint(
    db: &akasha_db::Db,
    worker_id: String,
    run_id: String,
    checkpoint: Value,
) -> Result<(), AppError> {
    let updated = workers::checkpoint(
        db,
        UpdateCheckpointInput {
            id: normalize_worker_id(worker_id)?,
            run_id: normalize_run_id(run_id)?,
            checkpoint,
        },
    )
    .await
    .map_err(|error| AppError::Internal(error.into()))?;

    ensure_current_run(updated)
}

pub(super) async fn heartbeat(
    db: &akasha_db::Db,
    worker_id: String,
    run_id: String,
) -> Result<(), AppError> {
    let updated = workers::heartbeat(
        db,
        normalize_worker_id(worker_id)?,
        normalize_run_id(run_id)?,
    )
    .await
    .map_err(|error| AppError::Internal(error.into()))?;

    ensure_current_run(updated)
}

pub(super) async fn complete(
    db: &akasha_db::Db,
    worker_id: String,
    run_id: String,
    phase: WorkerPhaseRequest,
    checkpoint: Value,
) -> Result<(), AppError> {
    let phase = match phase {
        WorkerPhaseRequest::InitialBackfill => WorkerPhase::InitialBackfill,
        WorkerPhaseRequest::Incremental => WorkerPhase::Incremental,
    };
    let updated = workers::complete(
        db,
        CompleteWorkerInput {
            id: normalize_worker_id(worker_id)?,
            run_id: normalize_run_id(run_id)?,
            phase,
            checkpoint,
        },
    )
    .await
    .map_err(|error| AppError::Internal(error.into()))?;

    ensure_current_run(updated)
}

pub(super) async fn fail(
    db: &akasha_db::Db,
    worker_id: String,
    run_id: String,
    error: String,
) -> Result<(), AppError> {
    let error = error.trim();
    if error.is_empty() {
        return Err(AppError::BadRequest("error must not be empty".into()));
    }
    let error = error.chars().take(MAX_ERROR_LENGTH).collect();

    let updated = workers::fail(
        db,
        normalize_worker_id(worker_id)?,
        normalize_run_id(run_id)?,
        error,
    )
    .await
    .map_err(|db_error| AppError::Internal(db_error.into()))?;

    ensure_current_run(updated)
}

fn normalize_key_segment(name: &str, value: String) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{name} must not be empty")));
    }
    if value.len() > MAX_KEY_SEGMENT_LENGTH {
        return Err(AppError::BadRequest(format!("{name} is too long")));
    }
    if value.contains(':') {
        return Err(AppError::BadRequest(format!("{name} must not contain ':'")));
    }

    Ok(value.to_owned())
}

fn normalize_worker_id(value: String) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_KEY_SEGMENT_LENGTH * 3 + 2 {
        return Err(AppError::BadRequest("invalid worker_id".into()));
    }
    Ok(value.to_owned())
}

fn normalize_run_id(value: String) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 64 {
        return Err(AppError::BadRequest("invalid run_id".into()));
    }
    Ok(value.to_owned())
}

fn build_worker_id(worker_type: &str, source_id: Option<&str>, game_id: &str) -> String {
    match source_id {
        Some(source_id) => format!("{worker_type}:{source_id}:{game_id}"),
        None => format!("{worker_type}:{game_id}"),
    }
}

fn ensure_current_run(updated: bool) -> Result<(), AppError> {
    if updated {
        Ok(())
    } else {
        Err(AppError::Conflict("worker run is no longer current".into()))
    }
}
