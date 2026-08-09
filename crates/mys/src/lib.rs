//! 米游社视频临时签名客户端

mod client;
mod error;
mod types;

pub use client::MysClient;
pub use error::{MysError, Result};
pub use types::{MysAuthKey, MysGame, MysVideoUrl, is_auth_key_valid, with_auth_key};
