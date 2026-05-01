pub mod entities;
mod enums;

use sea_orm::entity::prelude::DatabaseConnection;
use sea_orm::{ConnectOptions, Database};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

static SEAORM_POOL: OnceLock<DatabaseConnection> = OnceLock::new();

pub async fn init<P: AsRef<Path>>(path: P) {
    let p = path.as_ref();
    if let Some(parent) = p.parent()
        && !parent.is_dir()
    {
        tokio::fs::create_dir_all(parent)
            .await
            .expect("failed to create database directory");
    }
    let url = format!("sqlite://{}?mode=rwc", p.to_str().unwrap());

    let mut opt = ConnectOptions::new(url);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .sqlx_logging(false);

    let pool = Database::connect(opt)
        .await
        .expect("db connection should connect");

    let entities_path = format!(
        "{}::entities::*",
        module_path!().split("::").next().unwrap()
    );
    pool.get_schema_registry(entities_path.as_str())
        .sync(&pool)
        .await
        .expect("db registry failed");

    SEAORM_POOL.set(pool).expect("seaorm pool should be set");
}

pub fn pool() -> &'static DatabaseConnection {
    SEAORM_POOL.get().expect("seaorm pool should set")
}
