mod game;

use anyhow::Result;
use async_trait::async_trait;

use crawler_core::{CrawlerContext, CrawlerTask};
// 使用默认的 Game
// use crawler_core::Game;
// use game::DemoGameExt;
// 自定义新 Game
use game::Game;

pub struct DemoTask;

#[async_trait]
impl CrawlerTask for DemoTask {
    fn name(&self) -> &'static str {
        "demo"
    }
    fn display_name(&self) -> &'static str {
        "示例"
    }
    fn description(&self) -> &'static str {
        "示例"
    }

    async fn run(&self, _ctx: &CrawlerContext) -> Result<()> {
        for g in Game::ALL {
            println!("{:?}", g);
        }
        Ok(())
    }
}
