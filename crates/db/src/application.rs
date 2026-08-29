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
        GameDataCollection, GameDataEntry, GameDataListFilter, GameDataRawItem,
        ListGameDataRawFilter, SyncGameDataCollectionCommand, SyncGameDataCollectionResult,
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

impl ApplicationRepository for Db {
    async fn delete_audit_logs_before(
        &self,
        cutoff: DateTime<FixedOffset>,
    ) -> RepositoryResult<u64> {
        repositories::audit::delete_before(self, cutoff)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_games(&self) -> RepositoryResult<Vec<GameSummary>> {
        repositories::games::list(self)
            .await
            .map_err(RepositoryError::new)
    }

    async fn find_game(&self, game_id: &str) -> RepositoryResult<Option<GameSummary>> {
        repositories::games::find_by_id(self, game_id)
            .await
            .map_err(RepositoryError::new)
    }

    async fn find_game_cover(&self, game_id: &str) -> RepositoryResult<Option<String>> {
        repositories::games::find_cover_by_id(self, game_id)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_calendar_events(
        &self,
        filter: ListCalendarEventsFilter,
    ) -> RepositoryResult<Vec<CalendarEvent>> {
        repositories::calendar::list_events(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_game_versions(&self, game_id: &str) -> RepositoryResult<Vec<GameVersion>> {
        repositories::game_versions::list(self, game_id)
            .await
            .map_err(RepositoryError::new)
    }

    async fn sync_game_versions(
        &self,
        command: SyncGameVersionsCommand,
    ) -> RepositoryResult<SyncGameVersionsResult> {
        repositories::game_versions::sync(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    async fn sync_calendar_events(
        &self,
        command: SyncCalendarEventsCommand,
    ) -> RepositoryResult<SyncCalendarEventsResult> {
        repositories::calendar::sync(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_game_data_collections(
        &self,
        game_id: &str,
    ) -> RepositoryResult<Vec<GameDataCollection>> {
        repositories::game_data::list_collections(self, game_id)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_game_data(
        &self,
        filter: GameDataListFilter,
    ) -> RepositoryResult<(u64, Vec<GameDataEntry>)> {
        repositories::game_data::list(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    async fn find_game_data(
        &self,
        game_id: &str,
        collection: &str,
        id: &str,
    ) -> RepositoryResult<Option<GameDataEntry>> {
        repositories::game_data::find(self, game_id, collection, id)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_game_data_raw(
        &self,
        filter: ListGameDataRawFilter,
    ) -> RepositoryResult<(u64, Vec<GameDataRawItem>)> {
        repositories::game_data::list_raw(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    async fn sync_game_data_collection(
        &self,
        command: SyncGameDataCollectionCommand,
    ) -> RepositoryResult<SyncGameDataCollectionResult> {
        repositories::game_data::sync(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    async fn update_game_data_collection(
        &self,
        command: UpdateGameDataCollectionCommand,
    ) -> RepositoryResult<SyncGameDataCollectionResult> {
        repositories::game_data::update(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_ys_characters(
        &self,
        filter: YsCharacterListFilter,
    ) -> RepositoryResult<(u64, Vec<YsCharacter>)> {
        repositories::characters::list_ys(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_sr_characters(
        &self,
        filter: SrCharacterListFilter,
    ) -> RepositoryResult<(u64, Vec<SrCharacter>)> {
        repositories::characters::list_sr(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_zzz_characters(
        &self,
        filter: ZzzCharacterListFilter,
    ) -> RepositoryResult<(u64, Vec<ZzzCharacter>)> {
        repositories::characters::list_zzz(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_news_sources(&self, game_id: &str) -> RepositoryResult<Vec<NewsSource>> {
        repositories::news::list_sources(self, game_id)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_news_tags(
        &self,
        game_id: &str,
        source_id: &str,
    ) -> RepositoryResult<Vec<NewsTag>> {
        repositories::news_tags::list_tags(self, game_id, source_id)
            .await
            .map_err(RepositoryError::new)
    }

    async fn find_news_series(
        &self,
        game_id: &str,
        source_id: &str,
        tag_name: &str,
    ) -> RepositoryResult<Option<NewsSeries>> {
        repositories::news_tags::find_series(self, game_id, source_id, tag_name)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_news(&self, filter: ListNewsFilter) -> RepositoryResult<(u64, Vec<NewsSummary>)> {
        repositories::news::list(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_news_feed(&self, filter: NewsFeedFilter) -> RepositoryResult<Vec<NewsSummary>> {
        repositories::news::list_feed(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_news_raw(
        &self,
        filter: ListNewsRawFilter,
    ) -> RepositoryResult<(u64, Vec<NewsRawItem>)> {
        repositories::news::list_raw(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    async fn find_news(
        &self,
        game_id: &str,
        source_id: &str,
        news_id: &str,
    ) -> RepositoryResult<Option<NewsSummary>> {
        repositories::news::find_by_id(self, game_id, source_id, news_id)
            .await
            .map_err(RepositoryError::new)
    }

    async fn list_related_videos(
        &self,
        game_id: &str,
        source_id: &str,
        news_id: &str,
        tags: &[String],
        limit: u64,
    ) -> RepositoryResult<Vec<NewsSummary>> {
        repositories::news::list_related_videos(self, game_id, source_id, news_id, tags, limit)
            .await
            .map_err(RepositoryError::new)
    }

    async fn update_news(&self, command: UpdateNewsCommand) -> RepositoryResult<UpdateNewsResult> {
        repositories::news::update_news(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    async fn sync_news_tags(
        &self,
        command: SyncNewsTagsCommand,
    ) -> RepositoryResult<SyncNewsTagsResult> {
        repositories::news_tags::sync_news_tags(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    async fn replace_news_tags(&self, command: ReplaceNewsTagsCommand) -> RepositoryResult<()> {
        repositories::news::replace_news_tags(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    async fn replace_news_characters(
        &self,
        command: ReplaceNewsCharactersCommand,
    ) -> RepositoryResult<()> {
        repositories::news::replace_news_characters(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    async fn acquire_worker(
        &self,
        request: WorkerAcquireRequest,
    ) -> RepositoryResult<WorkerAcquireResult> {
        repositories::workers::acquire_worker(self, request)
            .await
            .map_err(RepositoryError::new)
    }

    async fn heartbeat_worker(&self, worker_id: String, run_id: String) -> RepositoryResult<bool> {
        repositories::workers::heartbeat_worker(self, worker_id, run_id)
            .await
            .map_err(RepositoryError::new)
    }

    async fn checkpoint_worker(
        &self,
        command: WorkerUpdateCheckpointCommand,
    ) -> RepositoryResult<bool> {
        repositories::workers::checkpoint_worker(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    async fn complete_worker(&self, command: WorkerCompleteCommand) -> RepositoryResult<bool> {
        repositories::workers::complete_worker(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    async fn fail_worker(
        &self,
        worker_id: String,
        run_id: String,
        error_message: String,
    ) -> RepositoryResult<bool> {
        repositories::workers::fail_worker(self, worker_id, run_id, error_message)
            .await
            .map_err(RepositoryError::new)
    }
}
