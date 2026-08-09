use akasha_application::workers::{WorkerLease, WorkerPhase};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 获取 worker 租约的 HTTP 请求体
#[derive(Deserialize)]
pub(crate) struct AcquireWorkerRequest {
    pub acquire_id: String,
    pub worker_type: String,
    pub source_id: Option<String>,
    pub game_id: String,
}

/// 成功获取 worker 租约后的 HTTP 响应
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

impl From<WorkerLease> for AcquireWorkerResponse {
    /// 将成功的应用层租约序列化为 worker API 响应
    fn from(lease: WorkerLease) -> Self {
        Self {
            worker_id: lease.worker_id,
            phase: lease.phase.as_str().to_owned(),
            status: lease.status.as_str().to_owned(),
            checkpoint: lease.checkpoint,
            run_id: lease.run_id,
            lease_until: lease.lease_until.to_rfc3339(),
            last_success_at: lease.last_success_at.map(|value| value.to_rfc3339()),
        }
    }
}

/// 更新 worker 检查点的 HTTP 请求体
#[derive(Deserialize)]
pub(crate) struct CheckpointWorkerRequest {
    pub worker_id: String,
    pub run_id: String,
    pub checkpoint: Value,
}

/// 续期 worker 租约的 HTTP 请求体
#[derive(Deserialize)]
pub(crate) struct HeartbeatWorkerRequest {
    pub worker_id: String,
    pub run_id: String,
}

/// 完成 worker run 的 HTTP 请求体
#[derive(Deserialize)]
pub(crate) struct CompleteWorkerRequest {
    pub worker_id: String,
    pub run_id: String,
    pub phase: WorkerPhaseRequest,
    pub checkpoint: Value,
}

/// HTTP 请求使用的 worker 同步阶段
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkerPhaseRequest {
    InitialBackfill,
    Incremental,
}

impl From<WorkerPhaseRequest> for WorkerPhase {
    /// 将 HTTP 阶段枚举转换为应用层阶段枚举
    fn from(phase: WorkerPhaseRequest) -> Self {
        match phase {
            WorkerPhaseRequest::InitialBackfill => Self::InitialBackfill,
            WorkerPhaseRequest::Incremental => Self::Incremental,
        }
    }
}

/// 标记 worker run 失败的 HTTP 请求体
#[derive(Deserialize)]
pub(crate) struct FailWorkerRequest {
    pub worker_id: String,
    pub run_id: String,
    pub error: String,
}
