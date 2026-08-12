use std::time::Duration;

use sea_orm::{
    ActiveValue::Set,
    ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    QueryFilter,
    sea_query::{ColumnDef, Index, IndexOrder, Table},
};

use crate::{entities::news, error::DbError, seed};

const LEGACY_NEWS_COVERS: &[(&str, &str)] = &[
    (
        "ys",
        "https://ys.mihoyo.com/main/_nuxt/img/holder.37207c1.jpg",
    ),
    (
        "sr",
        "https://webstatic.mihoyo.com/upload/op-public/2023/01/24/b74ae5e3a8e8b021b67ea26e27a215f2_184072581688764639.png",
    ),
    (
        "zzz",
        "https://webstatic.mihoyo.com/upload/op-public/2022/09/17/a425b5ccb44c72e342cf3a6e488dc445_771169193410538499.jpg",
    ),
    (
        "planet",
        "https://fastcdn.mihoyo.com/content-v2/hyg/159836/d693785a0c7bbc09bc5a343465264403_6948692753721779299.png",
    ),
];

/// 数据库连接池等待可用连接的最长时间
const DATABASE_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(8);

/// 单条数据库语句允许执行的最长时间
const DATABASE_STATEMENT_TIMEOUT: Duration = Duration::from_secs(30);

/// 建立 PostgreSQL 数据库连接所需的配置
#[derive(Clone, Default)]
pub struct DbOptions {
    /// PostgreSQL 主机名或地址
    pub pg_host: String,
    /// PostgreSQL 端口
    pub pg_port: String,
    /// PostgreSQL 用户名
    pub pg_user: String,
    /// PostgreSQL 密码，不实现 Debug 以避免误写入日志
    pub pg_password: String,
    /// PostgreSQL 数据库名
    pub pg_database: String,
}

/// 已初始化的 SeaORM 数据库及 schema 同步入口
#[derive(Debug, Clone)]
pub struct Db {
    conn: DatabaseConnection,
}

impl Db {
    /// 返回底层 SeaORM 数据库连接，供持久化适配器使用
    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }

    /// 连接数据库并同步 schema、约束、索引与必需种子数据
    pub async fn init(options: DbOptions) -> Result<Self, DbError> {
        let db = Self::connect(options).await?;
        db.sync_schema().await?;
        db.sync_column_constraints().await?;
        db.normalize_legacy_news_covers().await?;
        db.sync_indexes().await?;
        db.seed_required_data().await?;
        Ok(db)
    }

    /// 根据连接选项建立带连接池配置的 PostgreSQL 连接
    async fn connect(options: DbOptions) -> Result<Self, DbError> {
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            options.pg_user,
            options.pg_password,
            options.pg_host,
            options.pg_port,
            options.pg_database,
        );

        let mut connect_options = ConnectOptions::new(url);
        connect_options
            .max_connections(100)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(8))
            .acquire_timeout(DATABASE_ACQUIRE_TIMEOUT)
            .idle_timeout(Duration::from_secs(8))
            .statement_timeout(DATABASE_STATEMENT_TIMEOUT)
            .sqlx_logging(false);

        let connection = Database::connect(connect_options)
            .await
            .map_err(DbError::Connect)?;

        Ok(Self { conn: connection })
    }

    /// 根据已注册 Entity 同步数据库 schema
    async fn sync_schema(&self) -> Result<(), DbError> {
        let entity_registry_path = format!("{}::entities::*", env!("CARGO_CRATE_NAME"));

        self.conn
            .get_schema_registry(&entity_registry_path)
            .sync(&self.conn)
            .await
            .map_err(DbError::SyncSchema)
    }

    /// 将旧 worker 写入的占位封面归一化为 NULL
    async fn normalize_legacy_news_covers(&self) -> Result<(), DbError> {
        news::Entity::update_many()
            .set(news::ActiveModel {
                cover: Set(None),
                ..Default::default()
            })
            .filter(news::Column::Cover.eq(""))
            .exec(&self.conn)
            .await
            .map_err(DbError::NormalizeLegacyData)?;

        for (game_id, cover) in LEGACY_NEWS_COVERS {
            news::Entity::update_many()
                .set(news::ActiveModel {
                    cover: Set(None),
                    ..Default::default()
                })
                .filter(news::Column::GameId.eq(*game_id))
                .filter(news::Column::SourceId.eq("web_cn"))
                .filter(news::Column::Cover.eq(*cover))
                .exec(&self.conn)
                .await
                .map_err(DbError::NormalizeLegacyData)?;
        }

        Ok(())
    }

    /// 同步 Entity schema sync 不会修改的列约束
    async fn sync_column_constraints(&self) -> Result<(), DbError> {
        let alter_news_cover = Table::alter()
            .table("news")
            .modify_column(ColumnDef::new("cover").null())
            .to_owned();

        self.conn
            .execute(&alter_news_cover)
            .await
            .map_err(DbError::SyncSchema)?;

        Ok(())
    }

    /// 同步高频查询所需的二级索引
    async fn sync_indexes(&self) -> Result<(), DbError> {
        let news_publish_time_index = Index::create()
            .if_not_exists()
            .name("idx_news_game_source_publish_time")
            .table("news")
            .col("game_id")
            .col("source_id")
            .col(("publish_time", IndexOrder::Desc))
            .to_owned();

        let audit_created_at_index = Index::create()
            .if_not_exists()
            .name("idx_audit_logs_created_at")
            .table("audit_logs")
            .col("created_at")
            .to_owned();

        self.conn
            .execute(&news_publish_time_index)
            .await
            .map_err(DbError::SyncIndexes)?;

        self.conn
            .execute(&audit_created_at_index)
            .await
            .map_err(DbError::SyncIndexes)?;

        Ok(())
    }

    /// 写入服务启动必须存在的基础数据
    async fn seed_required_data(&self) -> Result<(), DbError> {
        seed::apply(&self.conn)
            .await
            .map_err(DbError::SeedRequiredData)
    }
}
