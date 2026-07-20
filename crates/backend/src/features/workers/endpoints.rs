use axum::{Json, extract::State, http::StatusCode};

use crate::{
    http::{error::AppError, extractors::DataWriteActor},
    state::AppState,
};

use super::{
    dto::{
        AcquireWorkerRequest, AcquireWorkerResponse, CheckpointWorkerRequest,
        CompleteWorkerRequest, FailWorkerRequest, HeartbeatWorkerRequest,
    },
    use_cases,
};

pub(crate) async fn acquire(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<AcquireWorkerRequest>,
) -> Result<Json<AcquireWorkerResponse>, AppError> {
    tracing::info!(actor = %actor.label(), worker_type = %body.worker_type, game_id = %body.game_id, "acquiring worker");
    use_cases::acquire(state.db(), body).await.map(Json)
}

pub(crate) async fn heartbeat(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<HeartbeatWorkerRequest>,
) -> Result<StatusCode, AppError> {
    tracing::debug!(actor = %actor.label(), worker_id = %body.worker_id, "renewing worker lease");
    use_cases::heartbeat(state.db(), body.worker_id, body.run_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn checkpoint(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<CheckpointWorkerRequest>,
) -> Result<StatusCode, AppError> {
    tracing::debug!(actor = %actor.label(), worker_id = %body.worker_id, "checkpointing worker");
    use_cases::checkpoint(state.db(), body.worker_id, body.run_id, body.checkpoint).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn complete(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<CompleteWorkerRequest>,
) -> Result<StatusCode, AppError> {
    tracing::info!(actor = %actor.label(), worker_id = %body.worker_id, "completing worker");
    use_cases::complete(
        state.db(),
        body.worker_id,
        body.run_id,
        body.phase,
        body.checkpoint,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn fail(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<FailWorkerRequest>,
) -> Result<StatusCode, AppError> {
    tracing::warn!(actor = %actor.label(), worker_id = %body.worker_id, "worker failed");
    use_cases::fail(state.db(), body.worker_id, body.run_id, body.error).await?;
    Ok(StatusCode::NO_CONTENT)
}
