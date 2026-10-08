use akasha_application::{
    audit::{AuditActorType, AuditContext},
    calendar::{CalendarEventInput, ListCalendarEventsFilter, SyncCalendarEventsCommand},
    characters::YsCharacterListFilter,
    game_data::{
        GameDataCollectionFilter, GameDataEntry, GameDataListFilter, ListGameDataSyncStateFilter,
        SyncGameDataCollectionCommand, UpdateGameDataCollectionCommand,
    },
    game_versions::{GameVersionInput, SyncGameVersionsCommand},
    news::{
        ListNewsFilter, NewsCharacter, NewsCharacterInput, NewsFeedFilter, NewsFilter, NewsOrder,
        UpdateNewsCommand, VideoPlayback,
    },
    search::TextQuery,
};
use chrono::{FixedOffset, TimeZone, Utc};
use sea_orm::{EntityTrait, PaginatorTrait};
use serde_json::json;

use super::*;
use crate::{
    entities::{audit_logs, games, news_sources},
    repositories,
};

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
        source_hash: Some(format!("hash-{id}")),
    }
}

fn calendar_time(month: u32, day: u32, hour: u32) -> chrono::DateTime<FixedOffset> {
    FixedOffset::east_opt(8 * 60 * 60)
        .expect("UTC+8 should be valid")
        .with_ymd_and_hms(2026, month, day, hour, 0, 0)
        .single()
        .expect("test calendar time should be valid")
}

#[tokio::test]
async fn initializes_sqlite_schema_and_seed_data() {
    let db = test_db().await;

    let game_count = games::Entity::find()
        .count(db.conn())
        .await
        .expect("seeded games should be queryable");
    assert_eq!(game_count, 8);

    let nodusfall = games::Entity::find_by_id("nodusfall")
        .one(db.conn())
        .await
        .expect("Nodusfall seed should be queryable")
        .expect("Nodusfall game should be seeded");
    assert_eq!(nodusfall.name_en, "Nodusfall");
    assert_eq!(nodusfall.name_zh, "源初之结");
    assert_eq!(nodusfall.index, 8);

    let source = news_sources::Entity::find_by_id(("web_cn".to_owned(), "nodusfall".to_owned()))
        .one(db.conn())
        .await
        .expect("Nodusfall web source seed should be queryable")
        .expect("Nodusfall web source should be seeded");
    assert_eq!(source.name, "官方网站");
    assert_eq!(source.index, 1);

    let source =
        news_sources::Entity::find_by_id(("web_os_en_us".to_owned(), "nodusfall".to_owned()))
            .one(db.conn())
            .await
            .expect("Nodusfall overseas English source seed should be queryable")
            .expect("Nodusfall overseas English source should be seeded");
    assert_eq!(source.name, "国际服官网（英语）");
    assert_eq!(source.index, 2);

    let source = news_sources::Entity::find_by_id(("web_os_zh_tw".to_owned(), "ys".to_owned()))
        .one(db.conn())
        .await
        .expect("Genshin overseas Traditional Chinese source seed should be queryable")
        .expect("Genshin overseas Traditional Chinese source should be seeded");
    assert_eq!(source.name, "国际服官网（繁体中文）");
    assert_eq!(source.index, 3);
}

#[tokio::test]
async fn synchronizes_game_data_collections_independently() {
    let db = test_db().await;

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
    let db = test_db().await;

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
async fn incrementally_updates_game_data_and_lists_sync_state() {
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
    let (total, raw) = repositories::game_data::list_sync_state(
        &db,
        ListGameDataSyncStateFilter {
            game_id: "ys".to_owned(),
            collection: "weapon".to_owned(),
            after_id: None,
            limit: 100,
        },
    )
    .await
    .expect("sync state should be queryable");
    assert_eq!(total, 1);
    assert_eq!(raw[0].id, "2001");
    assert_eq!(raw[0].source_hash.as_deref(), Some("new-hash"));
}

/// 验证新闻角色关联可以随新闻写入并从公开查询投影读取
#[tokio::test]
async fn writes_and_reads_news_character_links() {
    let db = test_db().await;

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
            video_playback: None,
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

/// 嵌入视频播放方式在数据库和应用模型之间保持一致
#[tokio::test]
async fn writes_and_reads_embed_video_playback() {
    let db = test_db().await;

    repositories::news::update_news(
        &db,
        UpdateNewsCommand {
            game_id: "ys".to_owned(),
            source_id: "web_os_zh_tw".to_owned(),
            id: "video-1".to_owned(),
            title: "嵌入影片".to_owned(),
            intro: None,
            publish_time: Utc::now().fixed_offset(),
            source_url: "https://genshin.hoyoverse.com/zh-tw/news/detail/video-1".to_owned(),
            cover: None,
            news_type: "video".to_owned(),
            video_url: Some("https://www.youtube.com/watch?v=test".to_owned()),
            video_playback: Some(VideoPlayback::Embed),
            video_duration_ms: None,
            tags: Vec::new(),
            characters: Some(Vec::new()),
            raw_data: json!({}),
            audit: audit_context(),
        },
    )
    .await
    .expect("embed video should be written");

    let summary = repositories::news::find_by_id(&db, "ys", "web_os_zh_tw", "video-1")
        .await
        .expect("embed video should be queryable")
        .expect("embed video should exist");
    assert_eq!(summary.video_playback, Some(VideoPlayback::Embed));
}

/// 验证重复提交完全相同的新闻不会产生物理更新或新审计记录
#[tokio::test]
async fn skips_unchanged_news_writes() {
    let db = test_db().await;
    let command = UpdateNewsCommand {
        game_id: "ys".to_owned(),
        source_id: "web_cn".to_owned(),
        id: "unchanged-news".to_owned(),
        title: "重复检查新闻".to_owned(),
        intro: Some("内容没有变化".to_owned()),
        publish_time: Utc::now().fixed_offset(),
        source_url: "https://example.com/unchanged-news".to_owned(),
        cover: None,
        news_type: "article".to_owned(),
        video_url: None,
        video_playback: None,
        video_duration_ms: None,
        tags: Vec::new(),
        characters: Some(Vec::new()),
        raw_data: json!({ "id": "unchanged-news" }),
        audit: audit_context(),
    };

    let created = repositories::news::update_news(&db, command.clone())
        .await
        .expect("news should be created");
    assert!(created.created);
    assert!(created.changed);
    let audit_count = audit_logs::Entity::find()
        .count(db.conn())
        .await
        .expect("audit logs should be countable");

    let unchanged = repositories::news::update_news(&db, command)
        .await
        .expect("identical news should be accepted");
    assert!(!unchanged.created);
    assert!(!unchanged.changed);
    assert_eq!(
        audit_logs::Entity::find()
            .count(db.conn())
            .await
            .expect("audit logs should remain countable"),
        audit_count
    );
}

/// 验证新闻列表和 RSS 共用标题语法、字面量匹配与角色筛选
#[tokio::test]
async fn filters_news_for_pages_and_feeds() {
    let db = test_db().await;

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
                video_playback: None,
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

#[tokio::test]
async fn synchronizes_versions_and_calendar_events_independently() {
    let db = test_db().await;

    let version_command = || SyncGameVersionsCommand {
        game_id: "ys".to_owned(),
        replace: true,
        versions: vec![
            GameVersionInput {
                id: "7.0".to_owned(),
                name: Some("无神怜爱的雪国".to_owned()),
                start_time: calendar_time(8, 12, 11),
                source_id: "mys".to_owned(),
                source_news_id: "version-70".to_owned(),
                source_hash: "version-hash-70".to_owned(),
            },
            GameVersionInput {
                id: "7.1".to_owned(),
                name: None,
                start_time: calendar_time(9, 23, 11),
                source_id: "mys".to_owned(),
                source_news_id: "version-71".to_owned(),
                source_hash: "version-hash-71".to_owned(),
            },
        ],
        audit: audit_context(),
    };
    let version_result = repositories::game_versions::sync(&db, version_command())
        .await
        .expect("game versions should synchronize");
    let event_result = repositories::calendar::sync(
        &db,
        SyncCalendarEventsCommand {
            game_id: "ys".to_owned(),
            replace: true,
            events: vec![CalendarEventInput {
                id: "event-1".to_owned(),
                kind: "game_activity".to_owned(),
                title: "新芽相助·初探雪原".to_owned(),
                start_time: calendar_time(8, 12, 11),
                end_time: calendar_time(8, 24, 4),
                version_id: Some("7.0".to_owned()),
                start_version_id: Some("7.0".to_owned()),
                cover: Some("https://example.com/event.jpg".to_owned()),
                labels: vec!["千星奇域".to_owned()],
                source_id: "mys".to_owned(),
                source_news_id: "news-1".to_owned(),
                source_url: "https://example.com/news-1".to_owned(),
                source_hash: "event-hash-1".to_owned(),
            }],
            audit: audit_context(),
        },
    )
    .await
    .expect("calendar events should synchronize");

    assert_eq!(version_result.versions_created, 2);
    assert_eq!(event_result.events_created, 1);
    let repeated_version_result = repositories::game_versions::sync(&db, version_command())
        .await
        .expect("unchanged game versions should synchronize without writes");
    assert!(!repeated_version_result.changed);
    assert_eq!(repeated_version_result.versions_created, 0);
    assert_eq!(repeated_version_result.versions_updated, 0);
    assert_eq!(repeated_version_result.versions_deleted, 0);
    let versions = repositories::game_versions::list(&db, "ys")
        .await
        .expect("versions should be queryable");
    assert_eq!(versions[0].end_time, Some(calendar_time(9, 23, 11)));
    assert_eq!(versions[1].end_time, None);
    let events = repositories::calendar::list_events(
        &db,
        ListCalendarEventsFilter {
            game_id: "ys".to_owned(),
            start_time: calendar_time(8, 1, 0),
            end_time: calendar_time(9, 1, 0),
            kinds: vec!["game_activity".to_owned()],
            limit: 20,
        },
    )
    .await
    .expect("events should be queryable");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].labels, ["千星奇域"]);
}

async fn test_db() -> Db {
    Db::init(DbOptions {
        sqlite_path: ":memory:".to_owned(),
    })
    .await
    .expect("SQLite schema and seed data should initialize")
}

#[tokio::test]
async fn achievement_projection_groups_filters_and_sync_state_agree() {
    use akasha_application::{ApplicationServices, achievements::AchievementListFilter};
    let db = Db::init(DbOptions {
        sqlite_path: ":memory:".to_owned(),
    })
    .await
    .expect("database");
    for game in ["ys", "sr", "zzz"] {
        let make = |id: &str, group: &str, order: i64, hidden: Option<bool>| {
            let mut entry = game_data_entry("achievement", id, id);
            entry.detail = None;
            entry.summary = json!({"id":id,"name":id,"description":"在山顶找到宝箱","group_id":group,"group_name":group,"group_order":order,"order":0,"hidden":hidden,"rewards":[],"target":null,"previous_id":null});
            entry
        };
        let service = ApplicationServices::new(db.clone());
        service
            .sync_game_data_collection(SyncGameDataCollectionCommand {
                game_id: game.to_owned(),
                collection: "achievement".to_owned(),
                items: vec![
                    make("1", "a", 2, Some(true)),
                    make("2", "b", 1, None),
                    make("3", "a", 2, Some(false)),
                ],
                audit: audit_context(),
            })
            .await
            .expect("normalized collection");
        let groups = service.list_achievement_groups(game).await.expect("groups");
        assert_eq!(
            groups
                .iter()
                .map(|g| (g.id.as_str(), g.total))
                .collect::<Vec<_>>(),
            vec![("b", 1), ("a", 2)]
        );
        let (total, filtered) = service
            .list_achievements(
                game.to_owned(),
                Some(TextQuery::parse("宝箱").expect("query")),
                AchievementListFilter {
                    group_id: Some("a".to_owned()),
                    hidden: Some(false),
                },
                1,
                0,
            )
            .await
            .expect("filtered list");
        assert_eq!(total, 1);
        assert_eq!(filtered[0].id, "3");
        let (_, visible) = service
            .list_achievements(
                game.to_owned(),
                None,
                AchievementListFilter {
                    group_id: None,
                    hidden: None,
                },
                1,
                0,
            )
            .await
            .expect("ordered list");
        assert_eq!(visible[0].id, "2");
        let (total, state) = service
            .list_game_data_sync_state(ListGameDataSyncStateFilter {
                game_id: game.to_owned(),
                collection: "achievement".to_owned(),
                after_id: Some("1".to_owned()),
                limit: 1,
            })
            .await
            .expect("sync state");
        assert_eq!(total, 3);
        assert_eq!(state[0].id, "2");
    }
}
