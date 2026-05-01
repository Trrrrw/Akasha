mod game;
pub mod http;

use anyhow::Result;
use async_trait::async_trait;
pub use tracing::{debug, error, info, warn};

pub use db::entities;
pub use game::Game;

#[derive(Default)]
pub struct CrawlerContext;

#[async_trait]
pub trait CrawlerTask {
    fn name(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    async fn run(&self, ctx: &CrawlerContext) -> Result<()>;
}
