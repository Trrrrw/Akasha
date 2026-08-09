use akasha_application::workers::{WorkerPhase, WorkerStatus};

use crate::{
    entities::worker_states,
    models::{WorkerPhase as WorkerPhaseRecord, WorkerStatus as WorkerStatusRecord},
};

pub(crate) use akasha_application::workers::WorkerState;

impl From<worker_states::Model> for WorkerState {
    /// 将 worker 状态 Entity 映射为应用层读取模型
    fn from(row: worker_states::Model) -> Self {
        Self {
            id: row.id,
            worker_type: row.worker_type,
            source_id: row.source_id,
            game_id: row.game_id,
            phase: map_phase(row.phase),
            status: map_status(row.status),
            checkpoint: row.checkpoint,
            run_id: row.run_id,
            lease_until: row.lease_until,
            last_error: row.last_error,
            last_success_at: row.last_success_at,
            updated_at: row.updated_at,
        }
    }
}

/// 将 SeaORM 枚举值转换为应用层 worker 阶段
fn map_phase(phase: WorkerPhaseRecord) -> WorkerPhase {
    match phase {
        WorkerPhaseRecord::InitialBackfill => WorkerPhase::InitialBackfill,
        WorkerPhaseRecord::Incremental => WorkerPhase::Incremental,
    }
}

/// 将 SeaORM 枚举值转换为应用层 worker 状态
fn map_status(status: WorkerStatusRecord) -> WorkerStatus {
    match status {
        WorkerStatusRecord::Idle => WorkerStatus::Idle,
        WorkerStatusRecord::Running => WorkerStatus::Running,
        WorkerStatusRecord::Failed => WorkerStatus::Failed,
    }
}
