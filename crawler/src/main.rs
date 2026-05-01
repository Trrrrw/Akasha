mod scheduler;
mod task_registry;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;
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
    Serve {
        #[arg(default_value = DEFAULT_CRON)]
        cron: String,
    },
    Run {
        task: String,
    },
    RunAll,
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();

    db::init("data/hoyo-info.db").await;

    let ctx = Arc::new(CrawlerContext);
    let registry = Arc::new(TaskRegistry::new());

    match cli.command.unwrap_or(Command::Serve {
        cron: DEFAULT_CRON.to_string(),
    }) {
        Command::Serve { cron } => scheduler::serve(registry, ctx, cron).await,
        Command::Run { task } => registry.run(&task, &ctx).await,
        Command::RunAll => registry.run_all(Arc::clone(&ctx)).await,
        Command::List => {
            for task in registry.names() {
                println!("{task}");
            }
            Ok(())
        }
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stdout)
        .init();
}
