use chrono::{DateTime, Utc};
use url::Url;

use crate::{MysError, Result};

/// 米游社支持的游戏分区
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MysGame {
    Bh3,
    Ys,
    Wd,
    Sr,
    Zzz,
    Hna,
    Planet,
}

impl MysGame {
    /// 将应用层游戏 ID 转换为米游社游戏分区
    pub fn from_game_id(game_id: &str) -> Result<Self> {
        match game_id {
            "bh3" => Ok(Self::Bh3),
            "ys" => Ok(Self::Ys),
            "wd" => Ok(Self::Wd),
            "sr" => Ok(Self::Sr),
            "zzz" => Ok(Self::Zzz),
            "hna" => Ok(Self::Hna),
            "planet" => Ok(Self::Planet),
            _ => Err(MysError::UnsupportedGame(game_id.to_owned())),
        }
    }

    /// 返回米游社详情接口使用的游戏分区编号
    pub const fn gids(self) -> u32 {
        match self {
            Self::Bh3 => 1,
            Self::Ys => 2,
            Self::Wd => 4,
            Self::Sr => 6,
            Self::Zzz => 8,
            Self::Hna => 9,
            Self::Planet => 10,
        }
    }
}

/// 米游社签发的视频临时签名
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysAuthKey {
    /// 可添加到视频地址 `auth_key` 查询参数的签名值
    pub value: String,
    /// 签名的失效时间
    pub expires_at: DateTime<Utc>,
}

/// 米游社返回的视频地址，签名地址带过期时间，静态地址不设置过期时间
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysVideoUrl {
    /// 官方返回的视频地址
    pub url: String,
    /// 视频地址中签名的失效时间，无签名静态地址为 None
    pub expires_at: Option<DateTime<Utc>>,
}

impl MysVideoUrl {
    /// 判断视频地址此刻是否仍然可以复用
    pub fn is_valid(&self) -> bool {
        self.expires_at
            .is_none_or(|expires_at| expires_at > Utc::now())
    }
}

impl MysAuthKey {
    /// 从现有签名解析其过期时间
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let expires_at = parse_expiration(&value)?;

        Ok(Self { value, expires_at })
    }

    /// 判断签名此刻是否仍有效
    pub fn is_valid(&self) -> bool {
        self.is_valid_at(Utc::now())
    }

    /// 按指定时间判断签名是否仍有效
    pub fn is_valid_at(&self, now: DateTime<Utc>) -> bool {
        self.expires_at > now
    }
}

/// 判断现有 auth_key 是否仍有效，格式错误时视为失效
pub fn is_auth_key_valid(auth_key: &str) -> bool {
    MysAuthKey::parse(auth_key).is_ok_and(|auth_key| auth_key.is_valid())
}

/// 将 auth_key 添加到裸视频地址，已有的 auth_key 会被替换
pub fn with_auth_key(video_url: &str, auth_key: &str) -> Result<String> {
    let mut url = Url::parse(video_url)?;
    let query_pairs = url
        .query_pairs()
        .filter(|(key, _)| key != "auth_key")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();

    url.query_pairs_mut()
        .clear()
        .extend_pairs(&query_pairs)
        .append_pair("auth_key", auth_key);

    Ok(url.into())
}

/// 从 auth_key 的第一个时间戳字段取得过期时间
fn parse_expiration(auth_key: &str) -> Result<DateTime<Utc>> {
    let timestamp = auth_key
        .split('-')
        .next()
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or(MysError::InvalidAuthKeyExpiration)?;

    DateTime::from_timestamp(timestamp, 0).ok_or(MysError::InvalidAuthKeyExpiration)
}
