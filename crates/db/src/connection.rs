use std::time::Duration;

use sea_orm::{
    ActiveValue::Set,
    ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    QueryFilter,
    sea_query::{Index, IndexOrder},
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

/// SQLite 等待其他写事务完成的最长时间
const DATABASE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// 建立 SQLite 数据库连接所需的配置
#[derive(Clone, Default)]
pub struct DbOptions {
    /// SQLite 数据库文件路径
    pub sqlite_path: String,
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
        db.normalize_legacy_news_covers().await?;
        db.sync_indexes().await?;
        db.seed_required_data().await?;
        Ok(db)
    }

    /// 根据连接选项建立带连接池配置的 SQLite 连接
    async fn connect(options: DbOptions) -> Result<Self, DbError> {
        let database_path = std::path::Path::new(&options.sqlite_path);
        if let Some(parent) = database_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(DbError::PrepareDirectory)?;
        }

        let in_memory = options.sqlite_path == ":memory:";
        let url = if in_memory {
            "sqlite::memory:".to_owned()
        } else {
            format!("sqlite://{}?mode=rwc", options.sqlite_path)
        };

        let mut connect_options = ConnectOptions::new(url);
        connect_options
            .max_connections(if in_memory { 1 } else { 4 })
            .min_connections(1)
            .connect_timeout(Duration::from_secs(8))
            .acquire_timeout(DATABASE_ACQUIRE_TIMEOUT)
            .idle_timeout(Duration::from_secs(8))
            .sqlx_logging(false);
        connect_options.map_sqlx_sqlite_opts(|options| {
            options
                .foreign_keys(true)
                .busy_timeout(DATABASE_BUSY_TIMEOUT)
                .pragma("journal_mode", "WAL")
                .pragma("synchronous", "NORMAL")
                .pragma("cache_size", "-8192")
        });

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
    pub(crate) async fn normalize_legacy_news_covers(&self) -> Result<(), DbError> {
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
    pub(crate) async fn seed_required_data(&self) -> Result<(), DbError> {
        seed::apply(&self.conn)
            .await
            .map_err(DbError::SeedRequiredData)
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{EntityTrait, PaginatorTrait};

    use super::*;
    use crate::entities::games;

    #[tokio::test]
    async fn initializes_sqlite_schema_and_seed_data() {
        let db = Db::init(DbOptions {
            sqlite_path: ":memory:".to_owned(),
        })
        .await
        .expect("SQLite schema and seed data should initialize");

        let game_count = games::Entity::find()
            .count(db.conn())
            .await
            .expect("seeded games should be queryable");
        assert_eq!(game_count, 7);
    }
}
