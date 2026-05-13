use anyhow::{Result, bail};
use chrono::{FixedOffset, Utc};
use db::entities::meta;
use std::sync::Arc;
use tracing::{error, info};

use super::tasks;
use crawler_core::{CrawlerContext, CrawlerTask};

const LAST_CRAWLER_AT_KEY: &str = "last_crawler_at";

pub struct TaskRegistry {
    tasks: Vec<Arc<dyn CrawlerTask + Send + Sync>>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            tasks: tasks::get(),
        }
    }

    pub fn infos(&self) -> Vec<Vec<String>> {
        self.tasks
            .iter()
            .map(|task| {
                vec![
                    task.name().to_string(),
                    task.display_name().to_string(),
                    task.description().to_string(),
                ]
            })
            .collect()
    }

    pub async fn run(&self, name: &str, ctx: &CrawlerContext) -> Result<()> {
        let task = self
            .tasks
            .iter()
            .find(|task| task.name() == name)
            .ok_or_else(|| anyhow::anyhow!("未知爬虫任务: {name}"))?;

        task.run(ctx).await?;
        update_last_crawler_at().await?;

        Ok(())
    }

    pub async fn run_all(&self, ctx: Arc<CrawlerContext>) -> Result<()> {
        let mut tasks = tokio::task::JoinSet::new();
        let mut failures = Vec::new();

        for task in &self.tasks {
            let task = Arc::clone(task);
            let ctx = Arc::clone(&ctx);
            let task_name = task.name();

            tasks.spawn(async move {
                info!(task = task_name, "爬虫任务开始");

                match task.run(&ctx).await {
                    Ok(()) => {
                        info!(task = task_name, "爬虫任务完成");
                        Ok(())
                    }
                    Err(err) => {
                        error!(task = task_name, error = %err, "爬虫任务失败");
                        Err(format!("{task_name}: {err:#}"))
                    }
                }
            });
        }

        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(err)) => failures.push(err),
                Err(err) => failures.push(format!("tokio task join error: {err}")),
            }
        }

        if failures.is_empty() {
            update_last_crawler_at().await?;
            Ok(())
        } else {
            bail!("爬虫任务执行失败:\n{}", failures.join("\n"));
        }
    }
}

async fn update_last_crawler_at() -> Result<()> {
    let offset =
        FixedOffset::east_opt(8 * 3600).ok_or_else(|| anyhow::anyhow!("无效的时区偏移"))?;
    let value = Utc::now()
        .with_timezone(&offset)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    info!(last_crawler_at = %value, "更新爬虫完成时间");
    meta::Entity::set_value(LAST_CRAWLER_AT_KEY, value).await?;
    let stored_value = meta::Entity::get_value_by_key(LAST_CRAWLER_AT_KEY).await?;
    info!(last_crawler_at = ?stored_value, "爬虫完成时间写入完成");

    Ok(())
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}
