use std::time::Duration;

use sea_orm::{
    ActiveValue::Set,
    ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    QueryFilter,
    sea_query::{Alias, Index, IndexOrder},
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
        let news_search_index = Index::create()
            .if_not_exists()
            .name("idx_news_game_source_publish_id_title")
            .table("news")
            .col("game_id")
            .col("source_id")
            .col(("publish_time", IndexOrder::Desc))
            .col(("id", IndexOrder::Desc))
            .col("title")
            .to_owned();

        let legacy_news_publish_time_index = Index::drop()
            .if_exists()
            .name("idx_news_game_source_publish_time")
            .table("news")
            .to_owned();

        let audit_created_at_index = Index::create()
            .if_not_exists()
            .name("idx_audit_logs_created_at")
            .table("audit_logs")
            .col("created_at")
            .to_owned();

        let ys_game_data_index =
            game_data_index("idx_ys_game_data_collection_name_id", "ys_game_data");
        let sr_game_data_index =
            game_data_index("idx_sr_game_data_collection_name_id", "sr_game_data");
        let zzz_game_data_index =
            game_data_index("idx_zzz_game_data_collection_name_id", "zzz_game_data");

        let ys_news_character_index = news_character_index(
            "idx_ys_news_characters_character",
            "ys_news_characters_link",
        );
        let sr_news_character_index = news_character_index(
            "idx_sr_news_characters_character",
            "sr_news_characters_link",
        );
        let zzz_news_character_index = news_character_index(
            "idx_zzz_news_characters_character",
            "zzz_news_characters_link",
        );

        self.conn
            .execute(&news_search_index)
            .await
            .map_err(DbError::SyncIndexes)?;

        self.conn
            .execute(&legacy_news_publish_time_index)
            .await
            .map_err(DbError::SyncIndexes)?;

        self.conn
            .execute(&audit_created_at_index)
            .await
            .map_err(DbError::SyncIndexes)?;

        for index in [
            ys_game_data_index,
            sr_game_data_index,
            zzz_game_data_index,
            ys_news_character_index,
            sr_news_character_index,
            zzz_news_character_index,
        ] {
            self.conn
                .execute(&index)
                .await
                .map_err(DbError::SyncIndexes)?;
        }

        Ok(())
    }

    /// 写入服务启动必须存在的基础数据
    pub(crate) async fn seed_required_data(&self) -> Result<(), DbError> {
        seed::apply(&self.conn)
            .await
            .map_err(DbError::SeedRequiredData)
    }
}

/// 为单个游戏的数据目录创建集合与名称排序索引
fn game_data_index(name: &str, table: &str) -> sea_orm::sea_query::IndexCreateStatement {
    Index::create()
        .if_not_exists()
        .name(name)
        .table(Alias::new(table))
        .col("collection")
        .col("name")
        .col("id")
        .to_owned()
}

/// 为单个游戏的新闻角色关联创建按角色反查新闻的索引
fn news_character_index(name: &str, table: &str) -> sea_orm::sea_query::IndexCreateStatement {
    Index::create()
        .if_not_exists()
        .name(name)
        .table(Alias::new(table))
        .col("game_id")
        .col("source_id")
        .col("character_id")
        .col("news_id")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use akasha_application::{
        audit::{AuditActorType, AuditContext},
        characters::YsCharacterListFilter,
        game_data::{
            GameDataCollectionFilter, GameDataEntry, GameDataListFilter, ListGameDataRawFilter,
            SyncGameDataCollectionCommand, UpdateGameDataCollectionCommand,
        },
        news::{
            ListNewsFilter, NewsCharacter, NewsCharacterInput, NewsFeedFilter, NewsFilter,
            NewsOrder, UpdateNewsCommand,
        },
        search::TextQuery,
    };
    use chrono::Utc;
    use sea_orm::{EntityTrait, PaginatorTrait};
    use serde_json::json;

    use super::*;
    use crate::{entities::games, repositories};

    fn audit_context() -> AuditContext {
        AuditContext {
            actor_type: AuditActorType::Worker,
            actor_id: Some("test-worker".to_owned()),
            operation: "test".to_owned(),
            request_id: None,
            ip_address: None,
            user_agent: None,
            metadata: json!({}),
        }
    }

    fn ys_character(id: &str, name: &str) -> GameDataEntry {
        GameDataEntry {
            collection: "character".to_owned(),
            id: id.to_owned(),
            name: Some(name.to_owned()),
            icon: None,
            summary: json!({
                "id": id,
                "name": name,
                "name_en": name,
                "name_ja": name,
                "name_ko": name,
                "description": "",
                "description_en": "",
                "icon_url": "",
                "release_date": null,
                "birthday_month": null,
                "birthday_day": null,
                "rarity": null,
                "weapon_type": null,
                "element": null,
                "constellation": null,
                "region": null,
                "affiliation": null,
                "title": null,
                "cv_zh": null,
                "cv_en": null,
                "cv_ja": null,
                "cv_ko": null,
                "base_hp": null,
                "base_atk": null,
                "base_def": null,
                "crit_rate": null,
                "crit_dmg": null,
                "elemental_mastery": null,
                "stamina_recovery": null,
                "special": false
            }),
            detail: Some(json!({})),
            assets: json!({}),
            raw_data: Some(json!({ "id": id, "name": name })),
            source_hash: Some(format!("hash-{id}")),
        }
    }

    fn game_data_entry(collection: &str, id: &str, name: &str) -> GameDataEntry {
        GameDataEntry {
            collection: collection.to_owned(),
            id: id.to_owned(),
            name: Some(name.to_owned()),
            icon: None,
            summary: json!({ "id": id, "name": name }),
            detail: Some(json!({ "description": name })),
            assets: json!({}),
            raw_data: Some(json!({ "id": id, "name": name })),
            source_hash: Some(format!("hash-{id}")),
        }
    }

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

    #[tokio::test]
    async fn synchronizes_game_data_collections_independently() {
        let db = Db::init(DbOptions {
            sqlite_path: ":memory:".to_owned(),
        })
        .await
        .expect("SQLite schema and seed data should initialize");

        for (collection, id, name) in [
            ("character", "1001", "测试角色"),
            ("weapon", "2001", "测试武器"),
        ] {
            repositories::game_data::sync(
                &db,
                SyncGameDataCollectionCommand {
                    game_id: "ys".to_owned(),
                    collection: collection.to_owned(),
                    items: vec![game_data_entry(collection, id, name)],
                    audit: audit_context(),
                },
            )
            .await
            .expect("game data collection should synchronize");
        }

        repositories::game_data::sync(
            &db,
            SyncGameDataCollectionCommand {
                game_id: "ys".to_owned(),
                collection: "character".to_owned(),
                items: vec![game_data_entry("character", "1002", "新测试角色")],
                audit: audit_context(),
            },
        )
        .await
        .expect("one collection should be replaceable without affecting another");

        let collections = repositories::game_data::list_collections(&db, "ys")
            .await
            .expect("collections should be queryable");
        assert_eq!(collections.len(), 2);
        assert_eq!(collections.iter().map(|item| item.total).sum::<u64>(), 2);

        let (weapon_total, weapons) = repositories::game_data::list(
            &db,
            GameDataListFilter {
                game_id: "ys".to_owned(),
                collection: "weapon".to_owned(),
                query: None,
                collection_filter: None,
                limit: 20,
                offset: 0,
            },
        )
        .await
        .expect("unrelated collection should remain queryable");
        assert_eq!(weapon_total, 1);
        assert_eq!(weapons[0].id, "2001");
    }

    #[tokio::test]
    async fn filters_characters_from_game_data_summary_fields() {
        let db = Db::init(DbOptions {
            sqlite_path: ":memory:".to_owned(),
        })
        .await
        .expect("SQLite schema and seed data should initialize");

        let mut hu_tao = ys_character("1001", "胡桃");
        hu_tao.summary["element"] = json!("Pyro");
        hu_tao.summary["weapon_type"] = json!("WEAPON_POLE");
        hu_tao.summary["birthday_month"] = json!(7);
        hu_tao.summary["birthday_day"] = json!(15);
        let mut nahida = ys_character("1002", "纳西妲");
        nahida.summary["element"] = json!("Dendro");
        nahida.summary["weapon_type"] = json!("WEAPON_CATALYST");

        repositories::game_data::sync(
            &db,
            SyncGameDataCollectionCommand {
                game_id: "ys".to_owned(),
                collection: "character".to_owned(),
                items: vec![hu_tao, nahida],
                audit: audit_context(),
            },
        )
        .await
        .expect("character collection should synchronize");

        let character_filter = YsCharacterListFilter {
            query: None,
            element: Some("Pyro".to_owned()),
            weapon_type: Some("WEAPON_POLE".to_owned()),
            rarity: None,
            region: None,
            affiliation: None,
            voice_actor: None,
            birthday_month: Some(7),
            birthday_day: Some(15),
            special: None,
            birthday_only: true,
            limit: 20,
            offset: 0,
        };
        let (total, characters) = repositories::game_data::list(
            &db,
            GameDataListFilter {
                game_id: "ys".to_owned(),
                collection: "character".to_owned(),
                query: None,
                collection_filter: Some(GameDataCollectionFilter::YsCharacter(character_filter)),
                limit: 20,
                offset: 0,
            },
        )
        .await
        .expect("character projection should filter JSON summary fields");

        assert_eq!(total, 1);
        assert_eq!(characters[0].id, "1001");
    }

    #[tokio::test]
    async fn incrementally_updates_game_data_and_lists_raw_state() {
        let db = Db::init(DbOptions {
            sqlite_path: ":memory:".to_owned(),
        })
        .await
        .expect("SQLite schema and seed data should initialize");

        repositories::game_data::sync(
            &db,
            SyncGameDataCollectionCommand {
                game_id: "ys".to_owned(),
                collection: "weapon".to_owned(),
                items: vec![
                    game_data_entry("weapon", "2001", "旧武器"),
                    game_data_entry("weapon", "2002", "待删除武器"),
                ],
                audit: audit_context(),
            },
        )
        .await
        .expect("initial collection should synchronize");

        let mut changed = game_data_entry("weapon", "2001", "新武器");
        changed.source_hash = Some("new-hash".to_owned());
        changed.raw_data = Some(json!({ "id": "2001", "name": "新武器" }));
        let result = repositories::game_data::update(
            &db,
            UpdateGameDataCollectionCommand {
                game_id: "ys".to_owned(),
                collection: "weapon".to_owned(),
                items: vec![changed],
                removed_ids: vec!["2002".to_owned()],
                audit: audit_context(),
            },
        )
        .await
        .expect("collection should update incrementally");

        assert_eq!(result.updated, 1);
        assert_eq!(result.deleted, 1);
        assert_eq!(result.total, 1);
        let (total, raw) = repositories::game_data::list_raw(
            &db,
            ListGameDataRawFilter {
                game_id: "ys".to_owned(),
                collection: "weapon".to_owned(),
                after_id: None,
                include_raw_data: true,
                limit: 100,
            },
        )
        .await
        .expect("raw state should be queryable");
        assert_eq!(total, 1);
        assert_eq!(raw[0].id, "2001");
        assert_eq!(raw[0].source_hash.as_deref(), Some("new-hash"));
        assert_eq!(
            raw[0].raw_data,
            Some(json!({ "id": "2001", "name": "新武器" }))
        );
    }

    /// 验证新闻角色关联可以随新闻写入并从公开查询投影读取
    #[tokio::test]
    async fn writes_and_reads_news_character_links() {
        let db = Db::init(DbOptions {
            sqlite_path: ":memory:".to_owned(),
        })
        .await
        .expect("SQLite schema and seed data should initialize");

        repositories::game_data::sync(
            &db,
            SyncGameDataCollectionCommand {
                game_id: "ys".to_owned(),
                collection: "character".to_owned(),
                items: vec![ys_character("character-1", "胡桃")],
                audit: audit_context(),
            },
        )
        .await
        .expect("character directory should be synchronized");

        repositories::news::update_news(
            &db,
            UpdateNewsCommand {
                game_id: "ys".to_owned(),
                source_id: "web_cn".to_owned(),
                id: "news-1".to_owned(),
                title: "胡桃测试新闻".to_owned(),
                intro: Some("<p>测试</p>".to_owned()),
                publish_time: Utc::now().fixed_offset(),
                source_url: "https://example.com/news-1".to_owned(),
                cover: None,
                news_type: "article".to_owned(),
                video_url: None,
                video_duration_ms: None,
                tags: Vec::new(),
                characters: Some(vec![NewsCharacterInput {
                    id: "character-1".to_owned(),
                    name: "胡桃".to_owned(),
                }]),
                raw_data: json!({}),
                audit: audit_context(),
            },
        )
        .await
        .expect("news and character link should be written");

        let summary = repositories::news::find_by_id(&db, "ys", "web_cn", "news-1")
            .await
            .expect("news should be queryable")
            .expect("news should exist");
        assert_eq!(
            summary.characters,
            vec![NewsCharacter {
                id: "character-1".to_owned(),
                name: "胡桃".to_owned(),
            }]
        );

        let sync_result = repositories::game_data::update(
            &db,
            UpdateGameDataCollectionCommand {
                game_id: "ys".to_owned(),
                collection: "character".to_owned(),
                items: vec![ys_character("character-2", "纳西妲")],
                removed_ids: vec!["character-1".to_owned()],
                audit: audit_context(),
            },
        )
        .await
        .expect("character directory should be synchronized");
        assert!(sync_result.changed);
        assert_eq!(sync_result.deleted, 1);

        let summary = repositories::news::find_by_id(&db, "ys", "web_cn", "news-1")
            .await
            .expect("news should remain queryable")
            .expect("news should still exist");
        assert!(summary.characters.is_empty());
    }

    /// 验证新闻列表和 RSS 共用标题语法、字面量匹配与角色筛选
    #[tokio::test]
    async fn filters_news_for_pages_and_feeds() {
        let db = Db::init(DbOptions {
            sqlite_path: ":memory:".to_owned(),
        })
        .await
        .expect("SQLite schema and seed data should initialize");

        repositories::game_data::sync(
            &db,
            SyncGameDataCollectionCommand {
                game_id: "ys".to_owned(),
                collection: "character".to_owned(),
                items: vec![ys_character("1001", "胡桃")],
                audit: audit_context(),
            },
        )
        .await
        .expect("character directory should be synchronized");

        for (id, title, characters) in [
            (
                "news-1",
                "Version UPDATE 100%_Done",
                Some(vec![NewsCharacterInput {
                    id: "1001".to_owned(),
                    name: "胡桃".to_owned(),
                }]),
            ),
            ("news-2", "version preview", None),
            ("news-3", "Version UPDATE 100xxDone", None),
        ] {
            repositories::news::update_news(
                &db,
                UpdateNewsCommand {
                    game_id: "ys".to_owned(),
                    source_id: "web_cn".to_owned(),
                    id: id.to_owned(),
                    title: title.to_owned(),
                    intro: None,
                    publish_time: Utc::now().fixed_offset(),
                    source_url: format!("https://example.com/{id}"),
                    cover: None,
                    news_type: "article".to_owned(),
                    video_url: None,
                    video_duration_ms: None,
                    tags: Vec::new(),
                    characters,
                    raw_data: json!({}),
                    audit: audit_context(),
                },
            )
            .await
            .expect("news should be written");
        }

        let filter = NewsFilter {
            game_id: "ys".to_owned(),
            source_id: "web_cn".to_owned(),
            title_query: Some(TextQuery::parse("update \"100%_Done\"").expect("valid query")),
            tags: Vec::new(),
            include_untagged: false,
            character_ids: vec!["1001".to_owned()],
            news_type: None,
            start_publish_time: None,
            end_publish_time: None,
        };
        let (total, page) = repositories::news::list(
            &db,
            ListNewsFilter {
                filter: filter.clone(),
                limit: 20,
                offset: 0,
                order: NewsOrder::Desc,
            },
        )
        .await
        .expect("news page should be queryable");
        let feed = repositories::news::list_feed(&db, NewsFeedFilter { filter, limit: 20 })
            .await
            .expect("news feed should be queryable");

        assert_eq!(total, 1);
        assert_eq!(
            page.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
            ["news-1"]
        );
        assert_eq!(
            feed.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
            ["news-1"]
        );
    }
}
