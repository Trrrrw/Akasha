use thiserror::Error;

/// 数据库连接、schema 同步和持久化操作可能返回的错误
#[derive(Error, Debug)]
pub enum DbError {
    #[error("数据库连接失败: {0}")]
    Connect(#[source] sea_orm::DbErr),

    #[error("数据库结构同步失败: {0}")]
    SyncSchema(#[source] sea_orm::DbErr),

    #[error("数据库索引同步失败: {0}")]
    SyncIndexes(#[source] sea_orm::DbErr),

    #[error("数据库遗留数据归一化失败: {0}")]
    NormalizeLegacyData(#[source] sea_orm::DbErr),

    #[error("数据库初始数据写入失败: {0}")]
    SeedRequiredData(#[source] sea_orm::DbErr),

    #[error("数据库查询失败: {0}")]
    Query(#[source] sea_orm::DbErr),
}
