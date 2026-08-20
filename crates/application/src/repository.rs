use std::future::Future;

use chrono::{DateTime, FixedOffset};

use crate::{
    RepositoryResult,
    auth::{AuthUser, CurrentUser, GithubUserProfile, RefreshTokenMetadata},
    characters::{
        SrCharacter, SrCharacterListFilter, YsCharacter, YsCharacterListFilter, ZzzCharacter,
        ZzzCharacterListFilter,
    },
    game_data::{
        GameDataCollection, GameDataEntry, GameDataListFilter, GameDataRawItem,
        ListGameDataRawFilter, SyncGameDataCollectionCommand, SyncGameDataCollectionResult,
        UpdateGameDataCollectionCommand,
    },
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

/// 所有 Akasha 应用服务所需的持久化操作
pub trait ApplicationRepository: Send + Sync {
    /// 删除创建时间早于截止时间的审计日志
    fn delete_audit_logs_before(
        &self,
        cutoff: DateTime<FixedOffset>,
    ) -> impl Future<Output = RepositoryResult<u64>> + Send;

    /// 列出全部已配置游戏及其新闻摘要
    fn list_games(&self) -> impl Future<Output = RepositoryResult<Vec<GameSummary>>> + Send;

    /// 按公开标识查找一个游戏
    fn find_game(
        &self,
        game_id: &str,
    ) -> impl Future<Output = RepositoryResult<Option<GameSummary>>> + Send;

    /// 查找游戏的默认封面资源
    fn find_game_cover(
        &self,
        game_id: &str,
    ) -> impl Future<Output = RepositoryResult<Option<String>>> + Send;

    /// 列出一个游戏已同步的数据集合
    fn list_game_data_collections(
        &self,
        game_id: &str,
    ) -> impl Future<Output = RepositoryResult<Vec<GameDataCollection>>> + Send;

    /// 列出一个游戏数据集合中的条目
    fn list_game_data(
        &self,
        filter: GameDataListFilter,
    ) -> impl Future<Output = RepositoryResult<(u64, Vec<GameDataEntry>)>> + Send;

    /// 查找一个游戏数据条目
    fn find_game_data(
        &self,
        game_id: &str,
        collection: &str,
        id: &str,
    ) -> impl Future<Output = RepositoryResult<Option<GameDataEntry>>> + Send;

    /// 分页读取游戏数据原始条目
    fn list_game_data_raw(
        &self,
        filter: ListGameDataRawFilter,
    ) -> impl Future<Output = RepositoryResult<(u64, Vec<GameDataRawItem>)>> + Send;

    /// 同步一个游戏的单个数据集合
    fn sync_game_data_collection(
        &self,
        command: SyncGameDataCollectionCommand,
    ) -> impl Future<Output = RepositoryResult<SyncGameDataCollectionResult>> + Send;

    /// 增量更新一个游戏数据集合
    fn update_game_data_collection(
        &self,
        command: UpdateGameDataCollectionCommand,
    ) -> impl Future<Output = RepositoryResult<SyncGameDataCollectionResult>> + Send;

    /// 列出符合分页筛选条件的原神角色
    fn list_ys_characters(
        &self,
        filter: YsCharacterListFilter,
    ) -> impl Future<Output = RepositoryResult<(u64, Vec<YsCharacter>)>> + Send;

    /// 列出符合分页筛选条件的星铁角色
    fn list_sr_characters(
        &self,
        filter: SrCharacterListFilter,
    ) -> impl Future<Output = RepositoryResult<(u64, Vec<SrCharacter>)>> + Send;

    /// 列出符合分页筛选条件的绝区零角色
    fn list_zzz_characters(
        &self,
        filter: ZzzCharacterListFilter,
    ) -> impl Future<Output = RepositoryResult<(u64, Vec<ZzzCharacter>)>> + Send;

    /// 列出一个游戏已配置的新闻来源
    fn list_news_sources(
        &self,
        game_id: &str,
    ) -> impl Future<Output = RepositoryResult<Vec<NewsSource>>> + Send;

    /// 列出一个游戏及来源下可见的新闻标签
    fn list_news_tags(
        &self,
        game_id: &str,
        source_id: &str,
    ) -> impl Future<Output = RepositoryResult<Vec<NewsTag>>> + Send;

    /// 查找包含至少一个视频的新闻标签剧集
    fn find_news_series(
        &self,
        game_id: &str,
        source_id: &str,
        tag_name: &str,
    ) -> impl Future<Output = RepositoryResult<Option<NewsSeries>>> + Send;

    /// 列出一页新闻
    fn list_news(
        &self,
        filter: ListNewsFilter,
    ) -> impl Future<Output = RepositoryResult<(u64, Vec<NewsSummary>)>> + Send;

    /// 读取固定发布时间倒序的 RSS 新闻，不执行总数统计
    fn list_news_feed(
        &self,
        filter: NewsFeedFilter,
    ) -> impl Future<Output = RepositoryResult<Vec<NewsSummary>>> + Send;

    /// 读取维护任务需要的原始新闻分页
    fn list_news_raw(
        &self,
        filter: ListNewsRawFilter,
    ) -> impl Future<Output = RepositoryResult<(u64, Vec<NewsRawItem>)>> + Send;

    /// 在游戏及来源范围内查找一条新闻
    fn find_news(
        &self,
        game_id: &str,
        source_id: &str,
        news_id: &str,
    ) -> impl Future<Output = RepositoryResult<Option<NewsSummary>>> + Send;

    /// 按标签相关度列出同一来源的视频
    fn list_related_videos(
        &self,
        game_id: &str,
        source_id: &str,
        news_id: &str,
        tags: &[String],
        limit: u64,
    ) -> impl Future<Output = RepositoryResult<Vec<NewsSummary>>> + Send;

    /// 创建或更新一条新闻及其标签关联
    fn update_news(
        &self,
        command: UpdateNewsCommand,
    ) -> impl Future<Output = RepositoryResult<UpdateNewsResult>> + Send;

    /// 同步一个来源的标签目录
    fn sync_news_tags(
        &self,
        command: SyncNewsTagsCommand,
    ) -> impl Future<Output = RepositoryResult<SyncNewsTagsResult>> + Send;

    /// 替换同一来源多条新闻的标签
    fn replace_news_tags(
        &self,
        command: ReplaceNewsTagsCommand,
    ) -> impl Future<Output = RepositoryResult<()>> + Send;

    /// 替换同一来源多条新闻的角色关联
    fn replace_news_characters(
        &self,
        command: ReplaceNewsCharactersCommand,
    ) -> impl Future<Output = RepositoryResult<()>> + Send;

    /// 创建或更新 GitHub 账号关联的本地用户
    fn upsert_github_user(
        &self,
        profile: GithubUserProfile,
    ) -> impl Future<Output = RepositoryResult<AuthUser>> + Send;

    /// 为用户保存一个 refresh token 哈希
    fn save_refresh_token(
        &self,
        user_id: uuid::Uuid,
        refresh_token_hash: String,
        metadata: RefreshTokenMetadata,
    ) -> impl Future<Output = RepositoryResult<()>> + Send;

    /// 替换有效 refresh token 并返回其用户
    fn rotate_refresh_token(
        &self,
        old_refresh_token_hash: String,
        new_refresh_token_hash: String,
        metadata: RefreshTokenMetadata,
    ) -> impl Future<Output = RepositoryResult<AuthUser>> + Send;

    /// 吊销存在的 refresh token
    fn revoke_refresh_token(
        &self,
        refresh_token_hash: String,
    ) -> impl Future<Output = RepositoryResult<()>> + Send;

    /// 查找 access token subject 代表的活跃用户
    fn find_current_user(
        &self,
        user_id: uuid::Uuid,
    ) -> impl Future<Output = RepositoryResult<Option<CurrentUser>>> + Send;

    /// 在当前状态允许时获取 worker 租约
    fn acquire_worker(
        &self,
        request: WorkerAcquireRequest,
    ) -> impl Future<Output = RepositoryResult<WorkerAcquireResult>> + Send;

    /// 续期指定 worker run 的租约
    fn heartbeat_worker(
        &self,
        worker_id: String,
        run_id: String,
    ) -> impl Future<Output = RepositoryResult<bool>> + Send;

    /// 保存检查点并续期指定 worker run 的租约
    fn checkpoint_worker(
        &self,
        command: WorkerUpdateCheckpointCommand,
    ) -> impl Future<Output = RepositoryResult<bool>> + Send;

    /// 完成指定 worker run
    fn complete_worker(
        &self,
        command: WorkerCompleteCommand,
    ) -> impl Future<Output = RepositoryResult<bool>> + Send;

    /// 将指定 worker run 标记为失败
    fn fail_worker(
        &self,
        worker_id: String,
        run_id: String,
        error_message: String,
    ) -> impl Future<Output = RepositoryResult<bool>> + Send;
}
