use std::time::{Duration, SystemTime, UNIX_EPOCH};

use md5::{Digest, Md5};
use rand::{RngExt, distr::Alphanumeric};
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, COOKIE, ORIGIN, REFERER, USER_AGENT};
use serde::Deserialize;
use url::Url;

use crate::{MysAuthKey, MysError, MysGame, MysVideoUrl, Result};

const DETAIL_ENDPOINT: &str = "https://bbs-api.miyoushe.com/post/wapi/getPostFull";
const APP_VERSION: &str = "2.102.0";
const DS_SALT: &str = "r3KppdID2yT6ht6P7MxzQykauJj0Cmtg";
const USER_AGENT_VALUE: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36 Edg/150.0.0.0";

/// 使用 MyS Cookie 换取视频临时签名的客户端
#[derive(Clone)]
pub struct MysClient {
    http_client: reqwest::Client,
    cookie: String,
    device_fp: String,
    device_id: String,
}

impl MysClient {
    /// 使用默认 HTTP 客户端创建签名客户端
    pub fn new(cookie: impl Into<String>) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()?;
        Self::with_http_client(http_client, cookie)
    }

    /// 使用调用方提供的 HTTP 客户端创建签名客户端
    pub fn with_http_client(
        http_client: reqwest::Client,
        cookie: impl Into<String>,
    ) -> Result<Self> {
        let cookie = cookie.into();
        let device_fp = cookie_value(&cookie, "DEVICEFP")
            .ok_or(MysError::MissingCookieValue("DEVICEFP"))?
            .to_owned();
        let device_id = cookie_value(&cookie, "_MHYUUID")
            .ok_or(MysError::MissingCookieValue("_MHYUUID"))?
            .to_owned();

        Ok(Self {
            http_client,
            cookie,
            device_fp,
            device_id,
        })
    }

    /// 按游戏和文章 ID 获取对应视频的当前有效地址
    pub async fn get_video_url(&self, game: MysGame, post_id: &str) -> Result<Option<MysVideoUrl>> {
        let response = self.request_post_detail(game, post_id).await?;
        let post = response.data.and_then(|data| data.post);
        let Some(post) = post else {
            return Ok(None);
        };

        let video_url = post
            .vod_list
            .first()
            .into_iter()
            .flat_map(|video| video.resolutions.iter())
            .filter(|resolution| !resolution.url.is_empty())
            .max_by_key(|resolution| resolution.bitrate)
            .map(|resolution| resolution.url.as_str())
            .map(parse_video_url)
            .transpose()?;

        Ok(video_url)
    }

    /// 请求指定米游社文章详情并由官方接口签发视频地址
    async fn request_post_detail(
        &self,
        game: MysGame,
        post_id: &str,
    ) -> Result<PostDetailResponse> {
        let mut url = Url::parse(DETAIL_ENDPOINT)?;
        url.query_pairs_mut()
            .append_pair("gids", &game.gids().to_string())
            .append_pair("post_id", post_id)
            .append_pair("read", "1");

        tracing::debug!(gids = game.gids(), post_id, "请求米游社视频签名");

        let response = self
            .http_client
            .get(url)
            .header(ACCEPT, "application/json, text/plain, */*")
            .header(ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9")
            .header(COOKIE, &self.cookie)
            .header(ORIGIN, "https://www.miyoushe.com")
            .header(REFERER, "https://www.miyoushe.com/")
            .header(USER_AGENT, USER_AGENT_VALUE)
            .header("DS", create_ds()?)
            .header("x-rpc-app_version", APP_VERSION)
            .header("x-rpc-client_type", "4")
            .header("x-rpc-device_fp", &self.device_fp)
            .header("x-rpc-device_id", &self.device_id)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(MysError::HttpStatus(response.status()));
        }

        let result: PostDetailResponse = response.json().await?;
        if result.retcode != 0 {
            return Err(MysError::Api {
                retcode: result.retcode,
                message: result.message,
            });
        }

        Ok(result)
    }
}

/// 从官方返回的视频地址解析临时签名和过期时间
fn parse_video_url(video_url: &str) -> Result<MysVideoUrl> {
    let parsed = Url::parse(video_url)?;
    let auth_key = parsed
        .query_pairs()
        .find_map(|(key, value)| (key == "auth_key").then(|| value.into_owned()))
        .ok_or(MysError::MissingAuthKey)?;
    let auth_key = MysAuthKey::parse(auth_key)?;

    Ok(MysVideoUrl {
        url: video_url.to_owned(),
        expires_at: auth_key.expires_at,
    })
}

/// 根据网页端规则生成请求详情接口所需的 DS 头
fn create_ds() -> Result<String> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let random: String = rand::rng()
        .sample_iter(Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();
    let source = format!("salt={DS_SALT}&t={timestamp}&r={random}");
    let hash = Md5::digest(source)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    Ok(format!("{timestamp},{random},{hash}"))
}

/// 从 Cookie 字符串取得一个键对应的值
fn cookie_value<'a>(cookie: &'a str, key: &str) -> Option<&'a str> {
    cookie.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == key).then_some(value)
    })
}

#[derive(Deserialize)]
struct PostDetailResponse {
    retcode: i64,
    #[serde(default)]
    message: String,
    data: Option<PostDetailData>,
}

#[derive(Deserialize)]
struct PostDetailData {
    post: Option<PostDetail>,
}

#[derive(Deserialize)]
struct PostDetail {
    #[serde(default)]
    vod_list: Vec<Vod>,
}

#[derive(Deserialize)]
struct Vod {
    #[serde(default)]
    resolutions: Vec<Resolution>,
}

#[derive(Deserialize)]
struct Resolution {
    #[serde(default)]
    url: String,
    #[serde(default)]
    bitrate: u64,
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use crate::{MysAuthKey, MysError, is_auth_key_valid, with_auth_key};

    use super::{cookie_value, parse_video_url};

    /// 从复合 Cookie 中读取目标键值
    #[test]
    fn reads_cookie_value() {
        let cookie = "stuid=1; DEVICEFP=abc123; _MHYUUID=device-id";

        assert_eq!(cookie_value(cookie, "DEVICEFP"), Some("abc123"));
        assert_eq!(cookie_value(cookie, "missing"), None);
    }

    /// 从签名地址解析 auth_key 过期时间
    #[test]
    fn parses_signed_video_url() {
        let video_url = "https://prod-vod-sign.miyoushe.com/oMTdXuBxWswlaE1IPnkoCCtCzIBEym0n2QXDB?quality=high&auth_key=1786415744-faf5a51be6-0-637fc125cb2524320c58e7c825e285cc";

        let video = parse_video_url(video_url).expect("video URL should be valid");

        assert_eq!(video.url, video_url);
        assert_eq!(
            video.expires_at,
            chrono::Utc
                .timestamp_opt(1_786_415_744, 0)
                .single()
                .expect("timestamp should be valid")
        );
    }

    /// 拒绝不含 auth_key 的视频地址
    #[test]
    fn rejects_video_url_without_auth_key() {
        let error = parse_video_url("https://prod-vod-sign.miyoushe.com/video")
            .expect_err("missing auth_key must fail");

        assert!(matches!(error, MysError::MissingAuthKey));
    }

    /// 根据 auth_key 中的时间戳判断有效期
    #[test]
    fn checks_auth_key_expiration() {
        let valid = MysAuthKey::parse("1-faf5a51be6-0-637fc125cb2524320c58e7c825e285cc")
            .expect("auth_key should be valid");
        let now = chrono::Utc
            .timestamp_opt(0, 0)
            .single()
            .expect("timestamp should be valid");

        assert!(valid.is_valid_at(now));
        assert!(!is_auth_key_valid("invalid"));
    }

    /// 向视频地址添加或替换 auth_key 参数
    #[test]
    fn adds_or_replaces_auth_key_in_video_url() {
        let url = with_auth_key(
            "https://prod-vod-sign.miyoushe.com/video?quality=high&auth_key=expired",
            "1786415744-faf5a51be6-0-637fc125cb2524320c58e7c825e285cc",
        )
        .expect("video URL should be valid");

        assert_eq!(
            url,
            "https://prod-vod-sign.miyoushe.com/video?quality=high&auth_key=1786415744-faf5a51be6-0-637fc125cb2524320c58e7c825e285cc"
        );
    }
}
