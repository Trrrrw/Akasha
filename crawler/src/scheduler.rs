use anyhow::Result;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use super::task_registry::TaskRegistry;
use crawler_core::CrawlerContext;

pub async fn serve(
    registry: Arc<TaskRegistry>,
    ctx: Arc<CrawlerContext>,
    cron: String,
) -> Result<()> {
    let mut scheduler = JobScheduler::new().await?;
    let job_registry = Arc::clone(&registry);
    let job_ctx = Arc::clone(&ctx);

    scheduler
        .add(Job::new_async(cron.as_str(), move |_uuid, _lock| {
            let registry = Arc::clone(&job_registry);
            let ctx = Arc::clone(&job_ctx);

            Box::pin(async move {
                if let Err(err) = registry.run_all(ctx).await {
                    error!(error = %err, "爬虫定时任务执行失败");
                }
            })
        })?)
        .await?;

    info!("爬虫启动后首次执行开始");
    if let Err(err) = registry.run_all(Arc::clone(&ctx)).await {
        error!(error = %err, "爬虫启动后首次执行失败");
    }
    info!("爬虫启动后首次执行完成");

    scheduler.start().await?;
    info!(%cron, "爬虫定时调度已启动");
    tokio::signal::ctrl_c().await?;
    scheduler.shutdown().await?;
    Ok(())
}
