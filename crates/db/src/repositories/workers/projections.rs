use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use crate::{
    entities::worker_states,
    models::{WorkerPhase, WorkerStatus},
};

#[derive(Debug, Clone)]
pub struct WorkerState {
    pub id: String,
    pub worker_type: String,
    pub source_id: Option<String>,
    pub game_id: String,
    pub phase: WorkerPhase,
    pub status: WorkerStatus,
    pub checkpoint: Value,
    pub run_id: Option<String>,
    pub lease_until: Option<DateTime<FixedOffset>>,
    pub last_error: Option<String>,
    pub last_success_at: Option<DateTime<FixedOffset>>,
    pub updated_at: DateTime<FixedOffset>,
}

impl From<worker_states::Model> for WorkerState {
    fn from(row: worker_states::Model) -> Self {
        Self {
            id: row.id,
            worker_type: row.worker_type,
            source_id: row.source_id,
            game_id: row.game_id,
            phase: row.phase,
            status: row.status,
            checkpoint: row.checkpoint,
            run_id: row.run_id,
            lease_until: row.lease_until,
            last_error: row.last_error,
            last_success_at: row.last_success_at,
            updated_at: row.updated_at,
        }
    }
}
