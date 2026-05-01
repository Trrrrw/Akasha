use anyhow::{Result, bail};
use chrono::{FixedOffset, Utc};
use db::entities::meta;
use std::sync::Arc;
use tokio::task::JoinSet;

use crawler_core::{CrawlerContext, CrawlerTask};
use crawler_miyoushe_news::NewsMiyousheTask;
use crawler_official_site_news::NewsOfficialSiteTask;

const LAST_CRAWLER_AT_KEY: &str = "last_crawler_at";

pub struct TaskRegistry {
    tasks: Vec<Arc<dyn CrawlerTask + Send + Sync>>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            tasks: vec![Arc::new(NewsOfficialSiteTask), Arc::new(NewsMiyousheTask)],
        }
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.tasks.iter().map(|task| task.name())
    }

    pub async fn run(&self, name: &str, ctx: &CrawlerContext) -> Result<()> {
        let task = self
            .tasks
            .iter()
            .find(|task| task.name() == name)
            .ok_or_else(|| anyhow::anyhow!("unknown task: {name}"))?;

        task.run(ctx).await?;
        update_last_crawler_at().await?;

        Ok(())
    }

    pub async fn run_all(&self, ctx: Arc<CrawlerContext>) -> Result<()> {
        let mut jobs = JoinSet::new();

        for task in &self.tasks {
            let task = Arc::clone(task);
            let ctx = Arc::clone(&ctx);
            let task_name = task.name();

            jobs.spawn(async move { (task_name, task.run(&ctx).await) });
        }

        let mut failures = Vec::new();

        while let Some(result) = jobs.join_next().await {
            match result {
                Ok((_, Ok(()))) => {}
                Ok((task_name, Err(err))) => {
                    failures.push(format!("{task_name}: {err:#}"));
                }
                Err(err) => {
                    failures.push(format!("task join failed: {err}"));
                }
            }
        }

        if failures.is_empty() {
            update_last_crawler_at().await?;
            Ok(())
        } else {
            bail!("crawler tasks failed:\n{}", failures.join("\n"));
        }
    }
}

async fn update_last_crawler_at() -> Result<()> {
    let offset = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| anyhow::anyhow!("invalid timezone offset"))?;
    let value = Utc::now()
        .with_timezone(&offset)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    meta::Entity::set_value(LAST_CRAWLER_AT_KEY, value).await?;

    Ok(())
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}
