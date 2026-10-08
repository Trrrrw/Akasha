//! 应用层持久化端口的 SeaORM 实现

use akasha_application::{
    ApplicationRepository, RepositoryError, RepositoryResult,
    calendar::{
        CalendarEvent, ListCalendarEventsFilter, SyncCalendarEventsCommand,
        SyncCalendarEventsResult,
    },
    characters::{
        SrCharacter, SrCharacterListFilter, YsCharacter, YsCharacterListFilter, ZzzCharacter,
        ZzzCharacterListFilter,
    },
    game_data::{
        GameDataCollection, GameDataEntry, GameDataListFilter, GameDataSyncState,
        ListGameDataSyncStateFilter, SyncGameDataCollectionCommand, SyncGameDataCollectionResult,
        UpdateGameDataCollectionCommand,
    },
    game_versions::{GameVersion, SyncGameVersionsCommand, SyncGameVersionsResult},
    games::GameSummary,
    news::{
        ListNewsFilter, ListNewsRawFilter, NewsFeedFilter, NewsRawItem, NewsSeries, NewsSource,
        NewsSummary, NewsTag, ReplaceNewsCharactersCommand, ReplaceNewsTagsCommand,
        SyncNewsTagsCommand, SyncNewsTagsResult, UpdateNewsCommand, UpdateNewsResult,
    },
    workers::{
        WorkerAcquireRequest, WorkerAcquireResult, WorkerCompleteCommand,
        WorkerUpdateCheckpointCommand,
    },
};
use chrono::{DateTime, FixedOffset};

use crate::{Db, repositories};

/// 统一 repository 适配器的错误边界，映射表保留每个用例的实际实现位置
macro_rules! repository_methods {
    ($(fn $name:ident(&self $(, $arg:ident: $ty:ty)*) -> $result:ty => $target:path;)*) => {
        $(async fn $name(&self $(, $arg: $ty)*) -> $result {
            $target(self $(, $arg)*).await.map_err(RepositoryError::new)
        })*
    };
}

impl ApplicationRepository for Db {
    repository_methods! {
        fn delete_audit_logs_before(&self, cutoff: DateTime<FixedOffset>) -> RepositoryResult<u64> => repositories::audit::delete_before;
        fn list_games(&self) -> RepositoryResult<Vec<GameSummary>> => repositories::games::list;
        fn find_game(&self, game_id: &str) -> RepositoryResult<Option<GameSummary>> => repositories::games::find_by_id;
        fn find_game_cover(&self, game_id: &str) -> RepositoryResult<Option<String>> => repositories::games::find_cover_by_id;
        fn list_calendar_events(&self, filter: ListCalendarEventsFilter) -> RepositoryResult<Vec<CalendarEvent>> => repositories::calendar::list_events;
        fn list_game_versions(&self, game_id: &str) -> RepositoryResult<Vec<GameVersion>> => repositories::game_versions::list;
        fn sync_game_versions(&self, command: SyncGameVersionsCommand) -> RepositoryResult<SyncGameVersionsResult> => repositories::game_versions::sync;
        fn sync_calendar_events(&self, command: SyncCalendarEventsCommand) -> RepositoryResult<SyncCalendarEventsResult> => repositories::calendar::sync;
        fn list_game_data_collections(&self, game_id: &str) -> RepositoryResult<Vec<GameDataCollection>> => repositories::game_data::list_collections;
        fn list_game_data(&self, filter: GameDataListFilter) -> RepositoryResult<(u64, Vec<GameDataEntry>)> => repositories::game_data::list;
        fn find_game_data(&self, game_id: &str, collection: &str, id: &str) -> RepositoryResult<Option<GameDataEntry>> => repositories::game_data::find;
        fn list_game_data_sync_state(&self, filter: ListGameDataSyncStateFilter) -> RepositoryResult<(u64, Vec<GameDataSyncState>)> => repositories::game_data::list_sync_state;
        fn list_achievement_groups(&self, game_id: &str) -> RepositoryResult<Vec<akasha_application::achievements::AchievementGroup>> => repositories::game_data::achievement_groups;
        fn sync_game_data_collection(&self, command: SyncGameDataCollectionCommand) -> RepositoryResult<SyncGameDataCollectionResult> => repositories::game_data::sync;
        fn update_game_data_collection(&self, command: UpdateGameDataCollectionCommand) -> RepositoryResult<SyncGameDataCollectionResult> => repositories::game_data::update;
        fn list_ys_characters(&self, filter: YsCharacterListFilter) -> RepositoryResult<(u64, Vec<YsCharacter>)> => repositories::characters::list_ys;
        fn list_sr_characters(&self, filter: SrCharacterListFilter) -> RepositoryResult<(u64, Vec<SrCharacter>)> => repositories::characters::list_sr;
        fn list_zzz_characters(&self, filter: ZzzCharacterListFilter) -> RepositoryResult<(u64, Vec<ZzzCharacter>)> => repositories::characters::list_zzz;
        fn list_news_sources(&self, game_id: &str) -> RepositoryResult<Vec<NewsSource>> => repositories::news::list_sources;
        fn list_news_tags(&self, game_id: &str, source_id: &str) -> RepositoryResult<Vec<NewsTag>> => repositories::news_tags::list_tags;
        fn find_news_series(&self, game_id: &str, source_id: &str, tag_name: &str) -> RepositoryResult<Option<NewsSeries>> => repositories::news_tags::find_series;
        fn list_news(&self, filter: ListNewsFilter) -> RepositoryResult<(u64, Vec<NewsSummary>)> => repositories::news::list;
        fn list_news_feed(&self, filter: NewsFeedFilter) -> RepositoryResult<Vec<NewsSummary>> => repositories::news::list_feed;
        fn list_news_raw(&self, filter: ListNewsRawFilter) -> RepositoryResult<(u64, Vec<NewsRawItem>)> => repositories::news::list_raw;
        fn find_news(&self, game_id: &str, source_id: &str, news_id: &str) -> RepositoryResult<Option<NewsSummary>> => repositories::news::find_by_id;
        fn list_related_videos(&self, game_id: &str, source_id: &str, news_id: &str, tags: &[String], limit: u64) -> RepositoryResult<Vec<NewsSummary>> => repositories::news::list_related_videos;
        fn update_news(&self, command: UpdateNewsCommand) -> RepositoryResult<UpdateNewsResult> => repositories::news::update_news;
        fn sync_news_tags(&self, command: SyncNewsTagsCommand) -> RepositoryResult<SyncNewsTagsResult> => repositories::news_tags::sync_news_tags;
        fn replace_news_tags(&self, command: ReplaceNewsTagsCommand) -> RepositoryResult<()> => repositories::news::replace_news_tags;
        fn replace_news_characters(&self, command: ReplaceNewsCharactersCommand) -> RepositoryResult<()> => repositories::news::replace_news_characters;
        fn acquire_worker(&self, request: WorkerAcquireRequest) -> RepositoryResult<WorkerAcquireResult> => repositories::workers::acquire_worker;
        fn heartbeat_worker(&self, worker_id: String, run_id: String) -> RepositoryResult<bool> => repositories::workers::heartbeat_worker;
        fn checkpoint_worker(&self, command: WorkerUpdateCheckpointCommand) -> RepositoryResult<bool> => repositories::workers::checkpoint_worker;
        fn complete_worker(&self, command: WorkerCompleteCommand) -> RepositoryResult<bool> => repositories::workers::complete_worker;
        fn fail_worker(&self, worker_id: String, run_id: String, error_message: String) -> RepositoryResult<bool> => repositories::workers::fail_worker;
    }
}
