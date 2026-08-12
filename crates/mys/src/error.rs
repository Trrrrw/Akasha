use reqwest::StatusCode;
use thiserror::Error;

/// 米游社临时签名请求的错误
#[derive(Debug, Error)]
pub enum MysError {
    #[error("Cookie 缺少 {0}")]
    MissingCookieValue(&'static str),

    #[error("不支持米游社游戏分区 {0}")]
    UnsupportedGame(String),

    #[error("米游社请求失败")]
    Request(#[from] reqwest::Error),

    #[error("系统时间早于 Unix epoch")]
    SystemClock(#[from] std::time::SystemTimeError),

    #[error("米游社请求返回 HTTP {0}")]
    HttpStatus(StatusCode),

    #[error("米游社接口返回错误 {retcode}: {message}")]
    Api { retcode: i64, message: String },

    #[error("视频地址格式无效")]
    InvalidVideoUrl(#[from] url::ParseError),

    #[error("auth_key 中的过期时间无效")]
    InvalidAuthKeyExpiration,
}

/// 米游社临时签名操作的结果
pub type Result<T> = std::result::Result<T, MysError>;
