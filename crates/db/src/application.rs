//! 应用层持久化端口的 SeaORM 实现

use akasha_application::{
    ApplicationRepository, RepositoryError, RepositoryResult,
    auth::{AuthUser, CurrentUser, GithubUserProfile, RefreshTokenMetadata},
    characters::{
        CharacterListFilter, CharacterSummary, SyncCharactersCommand, SyncCharactersResult,
    },
    games::GameSummary,
    news::{
        ListNewsFilter, ListNewsRawFilter, NewsRawItem, NewsSeries, NewsSource, NewsSummary,
        NewsTag, ReplaceNewsCharactersCommand, ReplaceNewsTagsCommand, SyncNewsTagsCommand,
        SyncNewsTagsResult, UpdateNewsCommand, UpdateNewsResult,
    },
    workers::{
        WorkerAcquireRequest, WorkerAcquireResult, WorkerCompleteCommand,
        WorkerUpdateCheckpointCommand,
    },
};
use chrono::{DateTime, FixedOffset};
use uuid::Uuid;

use crate::{Db, repositories};

impl ApplicationRepository for Db {
    /// 将过期审计日志清理委托给 SeaORM audit repository
    async fn delete_audit_logs_before(
        &self,
        cutoff: DateTime<FixedOffset>,
    ) -> RepositoryResult<u64> {
        repositories::audit::delete_before(self, cutoff)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将游戏列表读取委托给 SeaORM 游戏 repository
    async fn list_games(&self) -> RepositoryResult<Vec<GameSummary>> {
        repositories::games::list(self)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将游戏详情读取委托给 SeaORM 游戏 repository
    async fn find_game(&self, game_id: &str) -> RepositoryResult<Option<GameSummary>> {
        repositories::games::find_by_id(self, game_id)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将游戏封面读取委托给 SeaORM 游戏 repository
    async fn find_game_cover(&self, game_id: &str) -> RepositoryResult<Option<String>> {
        repositories::games::find_cover_by_id(self, game_id)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将角色分页查询委托给 SeaORM 角色 repository
    async fn list_characters(
        &self,
        filter: CharacterListFilter,
    ) -> RepositoryResult<(u64, Vec<CharacterSummary>)> {
        repositories::characters::list_characters(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将角色同步命令委托给 SeaORM repository
    async fn sync_characters(
        &self,
        command: SyncCharactersCommand,
    ) -> RepositoryResult<SyncCharactersResult> {
        repositories::characters::sync_characters(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将新闻来源列表读取委托给 SeaORM 新闻 repository
    async fn list_news_sources(&self, game_id: &str) -> RepositoryResult<Vec<NewsSource>> {
        repositories::news::list_sources(self, game_id)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将标签列表读取委托给 SeaORM 新闻标签 repository
    async fn list_news_tags(
        &self,
        game_id: &str,
        source_id: &str,
    ) -> RepositoryResult<Vec<NewsTag>> {
        repositories::news_tags::list_tags(self, game_id, source_id)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将标签剧集读取委托给 SeaORM 新闻标签 repository
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

    /// 将新闻分页查询委托给 SeaORM 新闻 repository
    async fn list_news(&self, filter: ListNewsFilter) -> RepositoryResult<(u64, Vec<NewsSummary>)> {
        repositories::news::list(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将维护任务的原始新闻读取委托给 SeaORM 新闻 repository
    async fn list_news_raw(
        &self,
        filter: ListNewsRawFilter,
    ) -> RepositoryResult<(u64, Vec<NewsRawItem>)> {
        repositories::news::list_raw(self, filter)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将新闻详情查询委托给 SeaORM 新闻 repository
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

    /// 将相关视频查询委托给 SeaORM 新闻 repository
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

    /// 将新闻 upsert 命令委托给 SeaORM 新闻 repository
    async fn update_news(&self, command: UpdateNewsCommand) -> RepositoryResult<UpdateNewsResult> {
        repositories::news::update_news(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将标签目录同步委托给 SeaORM 新闻标签 repository
    async fn sync_news_tags(
        &self,
        command: SyncNewsTagsCommand,
    ) -> RepositoryResult<SyncNewsTagsResult> {
        repositories::news_tags::sync_news_tags(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将新闻标签替换委托给 SeaORM 新闻 repository
    async fn replace_news_tags(&self, command: ReplaceNewsTagsCommand) -> RepositoryResult<()> {
        repositories::news::replace_news_tags(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将新闻角色关联替换委托给 SeaORM 新闻 repository
    async fn replace_news_characters(
        &self,
        command: ReplaceNewsCharactersCommand,
    ) -> RepositoryResult<()> {
        repositories::news::replace_news_characters(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将本地 GitHub 用户同步委托给 SeaORM 认证 repository
    async fn upsert_github_user(&self, profile: GithubUserProfile) -> RepositoryResult<AuthUser> {
        repositories::auth::upsert_github_user(self, profile)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将 refresh token 保存委托给 SeaORM 认证 repository
    async fn save_refresh_token(
        &self,
        user_id: Uuid,
        refresh_token_hash: String,
        metadata: RefreshTokenMetadata,
    ) -> RepositoryResult<()> {
        repositories::auth::save_refresh_token(self, user_id, refresh_token_hash, metadata)
            .await
            .map(|_| ())
            .map_err(RepositoryError::new)
    }

    /// 将 refresh token 轮换委托给 SeaORM 认证 repository
    async fn rotate_refresh_token(
        &self,
        old_refresh_token_hash: String,
        new_refresh_token_hash: String,
        metadata: RefreshTokenMetadata,
    ) -> RepositoryResult<AuthUser> {
        repositories::auth::rotate_refresh_token(
            self,
            old_refresh_token_hash,
            new_refresh_token_hash,
            metadata,
        )
        .await
        .map_err(RepositoryError::new)
    }

    /// 将 refresh token 吊销委托给 SeaORM 认证 repository
    async fn revoke_refresh_token(&self, refresh_token_hash: String) -> RepositoryResult<()> {
        repositories::auth::revoke_refresh_token(self, refresh_token_hash)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将当前用户查询委托给 SeaORM 认证 repository
    async fn find_current_user(&self, user_id: Uuid) -> RepositoryResult<Option<CurrentUser>> {
        repositories::auth::find_current_user(self, user_id)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将 worker 租约获取委托给 SeaORM worker repository
    async fn acquire_worker(
        &self,
        request: WorkerAcquireRequest,
    ) -> RepositoryResult<WorkerAcquireResult> {
        repositories::workers::acquire_worker(self, request)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将 worker 租约续期委托给 SeaORM worker repository
    async fn heartbeat_worker(&self, worker_id: String, run_id: String) -> RepositoryResult<bool> {
        repositories::workers::heartbeat_worker(self, worker_id, run_id)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将 worker 检查点保存委托给 SeaORM worker repository
    async fn checkpoint_worker(
        &self,
        command: WorkerUpdateCheckpointCommand,
    ) -> RepositoryResult<bool> {
        repositories::workers::checkpoint_worker(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将 worker 完成操作委托给 SeaORM worker repository
    async fn complete_worker(&self, command: WorkerCompleteCommand) -> RepositoryResult<bool> {
        repositories::workers::complete_worker(self, command)
            .await
            .map_err(RepositoryError::new)
    }

    /// 将 worker 失败记录委托给 SeaORM worker repository
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
