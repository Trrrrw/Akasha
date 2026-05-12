pub mod entities;
pub mod enums;

use sea_orm::entity::prelude::DatabaseConnection;
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DbErr, Statement};
use std::env;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::warn;

static SEAORM_POOL: OnceLock<DatabaseConnection> = OnceLock::new();

/// 初始化数据库(不建表)
pub async fn init() {
    let pool = connect().await;

    SEAORM_POOL.set(pool).expect("数据库连接池应只初始化一次");
}

/// 初始化数据库(建表)
pub async fn init_and_sync() {
    let pool = connect().await;

    sync_schema(&pool).await;

    SEAORM_POOL.set(pool).expect("数据库连接池应只初始化一次");
}

/// 等待数据库初始化完成
pub async fn wait_until_ready(interval: Duration) {
    loop {
        match schema_ready().await {
            Ok(()) => break,
            Err(err) => {
                warn!(error = %err, ?interval, "数据库表结构尚未就绪，稍后重试");
                tokio::time::sleep(interval).await;
            }
        }
    }
}

/// 获取数据库连接池
pub fn pool() -> &'static DatabaseConnection {
    SEAORM_POOL.get().expect("数据库连接池尚未初始化")
}

/// 连接数据库
async fn connect() -> DatabaseConnection {
    let mut opt = ConnectOptions::new(database_url());
    opt.max_connections(20)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .sqlx_logging(false);

    Database::connect(opt).await.expect("数据库连接失败")
}

async fn sync_schema(pool: &DatabaseConnection) {
    let entities_path = format!(
        "{}::entities::*",
        module_path!().split("::").next().unwrap()
    );
    pool.get_schema_registry(entities_path.as_str())
        .sync(pool)
        .await
        .expect("数据库表结构同步失败");
}

async fn schema_ready() -> Result<(), DbErr> {
    let conn = pool();

    conn.query_one_raw(Statement::from_string(
        conn.get_database_backend(),
        "SELECT 1 FROM games LIMIT 1",
    ))
    .await?;

    Ok(())
}

fn database_url() -> String {
    let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
    let user = env_required("POSTGRES_USER");
    let password = env_required("POSTGRES_PASSWORD");
    let database = env::var("POSTGRES_DB").unwrap_or_else(|_| "Akasha".to_string());

    format!("postgres://{user}:{password}@{host}:{port}/{database}")
}

fn env_required(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("缺少必需的环境变量 {key}"))
}
