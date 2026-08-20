use serde_json::Value;

use crate::{
    ApplicationError, ApplicationRepository, ApplicationServices,
    audit::AuditContext,
    characters::{SrCharacterListFilter, YsCharacterListFilter, ZzzCharacterListFilter},
    search::TextQuery,
};

/// 一个游戏的数据集合摘要
#[derive(Debug, Clone)]
pub struct GameDataCollection {
    pub id: String,
    pub total: u64,
}

/// 游戏数据目录中的单个条目
#[derive(Debug, Clone)]
pub struct GameDataEntry {
    pub collection: String,
    pub id: String,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub summary: Value,
    pub detail: Option<Value>,
    pub assets: Value,
    pub raw_data: Option<Value>,
    pub source_hash: Option<String>,
}

/// Worker 增量同步所需的原始条目状态
#[derive(Debug, Clone)]
pub struct GameDataRawItem {
    pub id: String,
    pub raw_data: Option<Value>,
    pub source_hash: Option<String>,
}

/// 按 ID 游标读取一个集合的原始条目
#[derive(Debug, Clone)]
pub struct ListGameDataRawFilter {
    pub game_id: String,
    pub collection: String,
    pub after_id: Option<String>,
    pub include_raw_data: bool,
    pub limit: u64,
}

/// 游戏数据集合分页筛选条件
#[derive(Debug, Clone)]
pub struct GameDataListFilter {
    pub game_id: String,
    pub collection: String,
    pub query: Option<TextQuery>,
    pub collection_filter: Option<GameDataCollectionFilter>,
    pub limit: u64,
    pub offset: u64,
}

/// 需要按集合类型执行的专属筛选
#[derive(Debug, Clone)]
pub enum GameDataCollectionFilter {
    YsCharacter(YsCharacterListFilter),
    SrCharacter(SrCharacterListFilter),
    ZzzCharacter(ZzzCharacterListFilter),
}

/// 替换一个游戏的单个数据集合
#[derive(Debug, Clone)]
pub struct SyncGameDataCollectionCommand {
    pub game_id: String,
    pub collection: String,
    pub items: Vec<GameDataEntry>,
    pub audit: AuditContext,
}

/// 增量写入变化条目并删除来源中已消失的条目
#[derive(Debug, Clone)]
pub struct UpdateGameDataCollectionCommand {
    pub game_id: String,
    pub collection: String,
    pub items: Vec<GameDataEntry>,
    pub removed_ids: Vec<String>,
    pub audit: AuditContext,
}

/// 单个游戏数据集合同步结果
#[derive(Debug, Clone, Copy)]
pub struct SyncGameDataCollectionResult {
    pub created: u64,
    pub updated: u64,
    pub deleted: u64,
    pub changed: bool,
    pub total: u64,
}

impl<R> ApplicationServices<R>
where
    R: ApplicationRepository,
{
    /// 列出一个游戏已同步的数据集合
    pub async fn list_game_data_collections(
        &self,
        game_id: &str,
    ) -> Result<Vec<GameDataCollection>, ApplicationError> {
        Ok(self.repository.list_game_data_collections(game_id).await?)
    }

    /// 列出一个游戏数据集合中的条目
    pub async fn list_game_data(
        &self,
        filter: GameDataListFilter,
    ) -> Result<(u64, Vec<GameDataEntry>), ApplicationError> {
        Ok(self.repository.list_game_data(filter).await?)
    }

    /// 查找一个游戏数据条目
    pub async fn find_game_data(
        &self,
        game_id: &str,
        collection: &str,
        id: &str,
    ) -> Result<Option<GameDataEntry>, ApplicationError> {
        Ok(self
            .repository
            .find_game_data(game_id, collection, id)
            .await?)
    }

    /// 分页读取一个集合的原始数据和来源指纹
    pub async fn list_game_data_raw(
        &self,
        filter: ListGameDataRawFilter,
    ) -> Result<(u64, Vec<GameDataRawItem>), ApplicationError> {
        Ok(self.repository.list_game_data_raw(filter).await?)
    }

    /// 同步一个游戏的单个数据集合
    pub async fn sync_game_data_collection(
        &self,
        command: SyncGameDataCollectionCommand,
    ) -> Result<SyncGameDataCollectionResult, ApplicationError> {
        Ok(self.repository.sync_game_data_collection(command).await?)
    }

    /// 增量更新一个游戏数据集合
    pub async fn update_game_data_collection(
        &self,
        command: UpdateGameDataCollectionCommand,
    ) -> Result<SyncGameDataCollectionResult, ApplicationError> {
        Ok(self.repository.update_game_data_collection(command).await?)
    }
}
