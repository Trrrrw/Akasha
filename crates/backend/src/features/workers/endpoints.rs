use akasha_application::workers::{
    AcquireWorkerCommand, WorkerCompleteCommand, WorkerUpdateCheckpointCommand,
};
use axum::{Json, extract::State, http::StatusCode};

use crate::{
    http::{error::AppError, extractors::DataWriteActor},
    state::AppState,
};

use super::dto::{
    AcquireWorkerRequest, AcquireWorkerResponse, CheckpointWorkerRequest, CompleteWorkerRequest,
    FailWorkerRequest, HeartbeatWorkerRequest,
};

/// 为受信任数据 worker 获取租约
pub(crate) async fn acquire(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<AcquireWorkerRequest>,
) -> Result<Json<AcquireWorkerResponse>, AppError> {
    tracing::info!(actor = %actor.label(), worker_type = %body.worker_type, game_id = %body.game_id, "acquiring worker");
    let lease = state
        .application()
        .acquire_worker(AcquireWorkerCommand {
            acquire_id: body.acquire_id,
            worker_type: body.worker_type,
            source_id: body.source_id,
            game_id: body.game_id,
        })
        .await?;

    Ok(Json(AcquireWorkerResponse::from(lease)))
}

/// 续期活跃 worker 租约
pub(crate) async fn heartbeat(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<HeartbeatWorkerRequest>,
) -> Result<StatusCode, AppError> {
    tracing::debug!(actor = %actor.label(), worker_id = %body.worker_id, "renewing worker lease");
    state
        .application()
        .heartbeat_worker(body.worker_id, body.run_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// 保存活跃 worker 检查点并续期其租约
pub(crate) async fn checkpoint(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<CheckpointWorkerRequest>,
) -> Result<StatusCode, AppError> {
    tracing::debug!(actor = %actor.label(), worker_id = %body.worker_id, "checkpointing worker");
    state
        .application()
        .checkpoint_worker(WorkerUpdateCheckpointCommand {
            worker_id: body.worker_id,
            run_id: body.run_id,
            checkpoint: body.checkpoint,
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// 完成一个活跃 worker run
pub(crate) async fn complete(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<CompleteWorkerRequest>,
) -> Result<StatusCode, AppError> {
    tracing::info!(actor = %actor.label(), worker_id = %body.worker_id, "completing worker");
    state
        .application()
        .complete_worker(WorkerCompleteCommand {
            worker_id: body.worker_id,
            run_id: body.run_id,
            phase: body.phase.into(),
            checkpoint: body.checkpoint,
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// 将活跃 worker run 标记为失败
pub(crate) async fn fail(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<FailWorkerRequest>,
) -> Result<StatusCode, AppError> {
    tracing::warn!(actor = %actor.label(), worker_id = %body.worker_id, "worker failed");
    state
        .application()
        .fail_worker(body.worker_id, body.run_id, body.error)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
