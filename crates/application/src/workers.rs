use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use crate::{ApplicationError, ApplicationRepository, ApplicationServices};

const MAX_KEY_SEGMENT_LENGTH: usize = 64;
const MAX_ERROR_LENGTH: usize = 4_000;

/// Worker 租约的同步阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerPhase {
    InitialBackfill,
    Incremental,
}

impl WorkerPhase {
    /// 返回该阶段稳定的序列化表示
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InitialBackfill => "initial_backfill",
            Self::Incremental => "incremental",
        }
    }
}

/// Worker 持久化的生命周期状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerStatus {
    Idle,
    Running,
    Failed,
}

impl WorkerStatus {
    /// 返回该状态稳定的序列化表示
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Failed => "failed",
        }
    }
}

/// 不依赖持久化实现的 worker 状态读取模型
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

/// 成功获取 worker 后返回的完整租约信息
#[derive(Debug, Clone)]
pub struct WorkerLease {
    pub worker_id: String,
    pub phase: WorkerPhase,
    pub status: WorkerStatus,
    pub checkpoint: Value,
    pub run_id: String,
    pub lease_until: DateTime<FixedOffset>,
    pub last_success_at: Option<DateTime<FixedOffset>>,
}

/// 请求一个 worker 执行租约
#[derive(Debug, Clone)]
pub struct AcquireWorkerCommand {
    pub acquire_id: String,
    pub worker_type: String,
    pub source_id: Option<String>,
    pub game_id: String,
}

/// 已规范化、可直接持久化的 worker 租约请求
#[derive(Debug, Clone)]
pub struct WorkerAcquireRequest {
    pub worker_id: String,
    pub run_id: String,
    pub worker_type: String,
    pub source_id: Option<String>,
    pub game_id: String,
}

/// 尝试获取 worker 租约的持久化结果
#[derive(Debug, Clone)]
pub enum WorkerAcquireResult {
    Acquired(WorkerState),
    Busy(WorkerState),
}

/// 更新 worker 检查点并续期租约
#[derive(Debug, Clone)]
pub struct WorkerUpdateCheckpointCommand {
    pub worker_id: String,
    pub run_id: String,
    pub checkpoint: Value,
}

/// 完成一个 worker run 并记录最终检查点
#[derive(Debug, Clone)]
pub struct WorkerCompleteCommand {
    pub worker_id: String,
    pub run_id: String,
    pub phase: WorkerPhase,
    pub checkpoint: Value,
}

impl<R> ApplicationServices<R>
where
    R: ApplicationRepository,
{
    /// 获取有效 worker 租约或报告已存在的活跃租约
    pub async fn acquire_worker(
        &self,
        command: AcquireWorkerCommand,
    ) -> Result<WorkerLease, ApplicationError> {
        // 先校验并规范化所有组成 worker 标识的外部输入
        let worker_type = normalize_key_segment("worker_type", command.worker_type)?;
        let game_id = normalize_key_segment("game_id", command.game_id)?;
        let run_id = normalize_run_id(command.acquire_id)?;
        let source_id = command
            .source_id
            .map(|source_id| normalize_key_segment("source_id", source_id))
            .transpose()?;

        if worker_type == "news" && source_id.is_none() {
            return Err(ApplicationError::InvalidInput(
                "source_id is required for news workers".into(),
            ));
        }

        // 使用规范化请求争夺租约，再将持久化结果转换为应用层语义
        let worker_id = build_worker_id(&worker_type, source_id.as_deref(), &game_id);
        let result = self
            .repository
            .acquire_worker(WorkerAcquireRequest {
                worker_id,
                run_id,
                worker_type,
                source_id,
                game_id,
            })
            .await?;

        match result {
            WorkerAcquireResult::Acquired(state) => WorkerLease::try_from(state),
            WorkerAcquireResult::Busy(state) => {
                let lease_until = state
                    .lease_until
                    .map(|lease_until| lease_until.to_rfc3339())
                    .unwrap_or_else(|| "unknown".into());
                Err(ApplicationError::Conflict(format!(
                    "worker {} is already running until {lease_until}",
                    state.id
                )))
            }
        }
    }

    /// 保存当前 worker run 的检查点
    pub async fn checkpoint_worker(
        &self,
        command: WorkerUpdateCheckpointCommand,
    ) -> Result<(), ApplicationError> {
        // 仅将合法且属于当前 run 的检查点交给持久化层
        let updated = self
            .repository
            .checkpoint_worker(WorkerUpdateCheckpointCommand {
                worker_id: normalize_worker_id(command.worker_id)?,
                run_id: normalize_run_id(command.run_id)?,
                checkpoint: command.checkpoint,
            })
            .await?;

        ensure_current_run(updated)
    }

    /// 续期当前 worker run 的租约
    pub async fn heartbeat_worker(
        &self,
        worker_id: String,
        run_id: String,
    ) -> Result<(), ApplicationError> {
        // 先规范化运行标识，避免无效值进入条件更新
        let updated = self
            .repository
            .heartbeat_worker(normalize_worker_id(worker_id)?, normalize_run_id(run_id)?)
            .await?;

        ensure_current_run(updated)
    }

    /// 使用最终阶段及检查点完成当前 worker run
    pub async fn complete_worker(
        &self,
        command: WorkerCompleteCommand,
    ) -> Result<(), ApplicationError> {
        // 完成状态迁移前校验 worker 与 run 标识
        let updated = self
            .repository
            .complete_worker(WorkerCompleteCommand {
                worker_id: normalize_worker_id(command.worker_id)?,
                run_id: normalize_run_id(command.run_id)?,
                phase: command.phase,
                checkpoint: command.checkpoint,
            })
            .await?;

        ensure_current_run(updated)
    }

    /// 使用长度受限的诊断信息标记当前 worker run 为失败
    pub async fn fail_worker(
        &self,
        worker_id: String,
        run_id: String,
        error_message: String,
    ) -> Result<(), ApplicationError> {
        // 截断诊断信息后再执行条件失败更新
        let error_message = normalize_error_message(error_message)?;
        let updated = self
            .repository
            .fail_worker(
                normalize_worker_id(worker_id)?,
                normalize_run_id(run_id)?,
                error_message,
            )
            .await?;

        ensure_current_run(updated)
    }
}

impl TryFrom<WorkerState> for WorkerLease {
    type Error = ApplicationError;

    /// 将已获取的 worker 状态转换为 API 调用方可依赖的租约
    fn try_from(state: WorkerState) -> Result<Self, Self::Error> {
        let run_id = state.run_id.ok_or_else(|| {
            ApplicationError::InvariantViolation("acquired worker has no run_id".into())
        })?;
        let lease_until = state.lease_until.ok_or_else(|| {
            ApplicationError::InvariantViolation("acquired worker has no lease_until".into())
        })?;

        Ok(Self {
            worker_id: state.id,
            phase: state.phase,
            status: state.status,
            checkpoint: state.checkpoint,
            run_id,
            lease_until,
            last_success_at: state.last_success_at,
        })
    }
}

/// 校验复合 worker 标识中的一个字段
fn normalize_key_segment(name: &str, value: String) -> Result<String, ApplicationError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ApplicationError::InvalidInput(format!(
            "{name} must not be empty"
        )));
    }
    if value.len() > MAX_KEY_SEGMENT_LENGTH {
        return Err(ApplicationError::InvalidInput(format!(
            "{name} is too long"
        )));
    }
    if value.contains(':') {
        return Err(ApplicationError::InvalidInput(format!(
            "{name} must not contain ':'"
        )));
    }

    Ok(value.to_owned())
}

/// 校验先前 acquire 调用返回的完整 worker 标识
fn normalize_worker_id(value: String) -> Result<String, ApplicationError> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_KEY_SEGMENT_LENGTH * 3 + 2 {
        return Err(ApplicationError::InvalidInput("invalid worker_id".into()));
    }

    Ok(value.to_owned())
}

/// 校验不透明的 worker run 标识
fn normalize_run_id(value: String) -> Result<String, ApplicationError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 64 {
        return Err(ApplicationError::InvalidInput("invalid run_id".into()));
    }

    Ok(value.to_owned())
}

/// 构建用于租约协调的稳定 worker 标识
fn build_worker_id(worker_type: &str, source_id: Option<&str>, game_id: &str) -> String {
    match source_id {
        Some(source_id) => format!("{worker_type}:{source_id}:{game_id}"),
        None => format!("{worker_type}:{game_id}"),
    }
}

/// 必要时将条件更新结果转换为租约冲突错误
fn ensure_current_run(updated: bool) -> Result<(), ApplicationError> {
    if updated {
        Ok(())
    } else {
        Err(ApplicationError::Conflict(
            "worker run is no longer current".into(),
        ))
    }
}

/// 修剪并限制写入数据库的 worker 失败诊断信息长度
fn normalize_error_message(value: String) -> Result<String, ApplicationError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ApplicationError::InvalidInput(
            "error must not be empty".into(),
        ));
    }

    Ok(value.chars().take(MAX_ERROR_LENGTH).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 在调用持久化端口前拒绝空 worker 标识字段
    #[test]
    fn normalize_key_segment_rejects_empty_values() {
        let error = normalize_key_segment("game_id", "  ".into()).unwrap_err();

        assert!(
            matches!(error, ApplicationError::InvalidInput(message) if message == "game_id must not be empty")
        );
    }

    /// 保证来源级和游戏级 worker 标识保持稳定
    #[test]
    fn build_worker_id_uses_the_expected_segments() {
        assert_eq!(
            build_worker_id("news", Some("web_cn"), "ys"),
            "news:web_cn:ys"
        );
        assert_eq!(build_worker_id("calendar", None, "ys"), "calendar:ys");
    }

    /// 在持久化安全上限处截断失败诊断信息
    #[test]
    fn normalize_error_message_truncates_long_values() {
        let error_message = "x".repeat(MAX_ERROR_LENGTH + 1);
        let normalized = normalize_error_message(error_message).unwrap();

        assert_eq!(normalized.len(), MAX_ERROR_LENGTH);
    }
}
