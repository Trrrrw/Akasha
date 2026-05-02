mod scheduler;
mod task_registry;
mod tasks;

use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{ContentArrangement::Dynamic, Table, presets::UTF8_FULL};
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{EnvFilter, fmt};

use crawler_core::CrawlerContext;
use task_registry::TaskRegistry;

const DEFAULT_CRON: &str = "0 0/10 * * * *";

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// 启动定时任务
    Serve {
        #[arg(default_value = DEFAULT_CRON)]
        cron: String,
    },
    /// 运行单个任务
    Run {
        /// 任务名
        task: String,
    },
    /// 运行所有任务
    RunAll,
    /// 查看任务列表
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    // 初始化数据库
    db::init().await;
    db::wait_until_ready(Duration::from_secs(3)).await;

    let ctx = Arc::new(CrawlerContext);
    let registry = Arc::new(TaskRegistry::new());

    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Serve {
        cron: DEFAULT_CRON.to_string(),
    }) {
        Command::Serve { cron } => scheduler::serve(registry, ctx, cron).await,
        Command::Run { task } => registry.run(&task, &ctx).await,
        Command::RunAll => registry.run_all(Arc::clone(&ctx)).await,
        Command::List => {
            let mut table = Table::new();
            table.set_header(vec!["ID", "名称", "简介"]);
            table
                .load_preset(UTF8_FULL)
                .set_content_arrangement(Dynamic);
            for task_info in registry.infos() {
                table.add_row(task_info);
            }
            println!("{table}");
            Ok(())
        }
    }
}

/// 初始化日志
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tokio_cron_scheduler=warn"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stdout)
        .init();
}
