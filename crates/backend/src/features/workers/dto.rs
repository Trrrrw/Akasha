use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
pub(crate) struct AcquireWorkerRequest {
    pub acquire_id: String,
    pub worker_type: String,
    pub source_id: Option<String>,
    pub game_id: String,
}

#[derive(Serialize)]
pub(crate) struct AcquireWorkerResponse {
    pub worker_id: String,
    pub phase: String,
    pub status: String,
    pub checkpoint: Value,
    pub run_id: String,
    pub lease_until: String,
    pub last_success_at: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct CheckpointWorkerRequest {
    pub worker_id: String,
    pub run_id: String,
    pub checkpoint: Value,
}

#[derive(Deserialize)]
pub(crate) struct HeartbeatWorkerRequest {
    pub worker_id: String,
    pub run_id: String,
}

#[derive(Deserialize)]
pub(crate) struct CompleteWorkerRequest {
    pub worker_id: String,
    pub run_id: String,
    pub phase: WorkerPhaseRequest,
    pub checkpoint: Value,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkerPhaseRequest {
    InitialBackfill,
    Incremental,
}

#[derive(Deserialize)]
pub(crate) struct FailWorkerRequest {
    pub worker_id: String,
    pub run_id: String,
    pub error: String,
}
