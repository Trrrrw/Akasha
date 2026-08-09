//! Akasha 的应用服务、功能模型和持久化端口
//!
//! 本 crate 不依赖 HTTP 或 SeaORM，负责定义用例输入输出、校验规则，以及由基础设施适配器实现的 repository 协议

mod error;
mod repository;

pub mod audit;
pub mod auth;
pub mod characters;
pub mod games;
pub mod news;
pub mod workers;

pub use error::{ApplicationError, RepositoryError, RepositoryResult};
pub use repository::ApplicationRepository;

/// 通过一个持久化实现协调所有应用用例
#[derive(Clone, Debug)]
pub struct ApplicationServices<R> {
    repository: R,
}

impl<R> ApplicationServices<R> {
    /// 使用给定的 repository 实现创建应用服务
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}
