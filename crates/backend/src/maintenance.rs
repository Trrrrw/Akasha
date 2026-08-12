use std::time::Duration;

use akasha_application::ApplicationServices;
use akasha_db::Db;
use chrono::Utc;

const CLEANUP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// 启动每日清理过期审计日志的后台任务
pub(crate) fn spawn_audit_log_cleanup(application: ApplicationServices<Db>, retention_days: u32) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);

        loop {
            // interval 第一次触发会立即执行，确保服务启动后及时清理
            interval.tick().await;

            let cutoff =
                Utc::now().fixed_offset() - chrono::Duration::days(i64::from(retention_days));

            match application.delete_audit_logs_before(cutoff).await {
                Ok(deleted) if deleted > 0 => {
                    tracing::info!(deleted, retention_days, "removed expired audit logs");
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::error!(
                        ?error,
                        retention_days,
                        "failed to remove expired audit logs"
                    );
                }
            }
        }
    });
}
