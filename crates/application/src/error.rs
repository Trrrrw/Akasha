use std::error::Error;

use thiserror::Error;

/// 持久化适配器报告的失败，不暴露其具体实现类型
#[derive(Debug, Error)]
#[error("persistence operation failed")]
pub struct RepositoryError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl RepositoryError {
    /// 将基础设施错误包装为应用服务可使用的错误
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

/// Repository 端口返回的统一结果类型
pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// 具有应用层语义、可由交付层映射为响应的失败
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// 提供的命令参数无效
    #[error("{0}")]
    InvalidInput(String),

    /// 请求的状态迁移与当前状态冲突
    #[error("{0}")]
    Conflict(String),

    /// 持久化结果违反了用例所需的不变量
    #[error("{0}")]
    InvariantViolation(String),

    /// 持久化适配器在执行用例时失败
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}
