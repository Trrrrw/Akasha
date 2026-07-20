use std::sync::Arc;

use akasha_db::Db;
use anyhow::Result;

use crate::Config;

#[derive(Clone)]
pub struct AppState {
    config: Arc<Config>,
    db: Db,
    http_client: reqwest::Client,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        let db = Db::init(config.database.clone()).await?;
        let http_client = reqwest::Client::builder()
            .user_agent("akasha-backend")
            .build()?;

        Ok(Self {
            config: Arc::new(config),
            db,
            http_client,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }
}
