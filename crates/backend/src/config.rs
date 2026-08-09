use std::{
    env,
    net::{IpAddr, SocketAddr},
};

use akasha_db::DbOptions;
use anyhow::{Context, Result, bail};

/// 从进程环境变量读取的运行时配置
#[derive(Clone)]
pub struct Config {
    /// HTTP 服务监听地址
    pub bind_addr: SocketAddr,
    /// 对外公开的静态资源根地址
    pub asset_base_url: String,
    /// 米游社请求视频签名的 Cookie，未配置时仅禁用签名
    pub mys_cookie: Option<String>,
    /// 数据库连接配置
    pub database: DbOptions,
    /// 应用 token 配置
    pub auth: AuthConfig,
    /// GitHub OAuth 配置
    pub github: GithubConfig,
    /// 数据 worker 认证配置
    pub worker: WorkerConfig,
    /// 新闻公开接口限流配置
    pub public_rate_limits: PublicRateLimitConfig,
}

/// 用于签发和验证应用 token 的密钥
#[derive(Clone)]
pub struct AuthConfig {
    /// access token 的 HMAC 密钥
    pub jwt_secret: String,
    /// refresh token 等敏感值的哈希密钥
    pub token_hash_secret: String,
}

/// GitHub OAuth 客户端配置
#[derive(Clone)]
pub struct GithubConfig {
    /// GitHub OAuth 应用客户端 ID
    pub client_id: String,
    /// GitHub OAuth 应用客户端密钥
    pub client_secret: String,
    /// GitHub 登录完成后的回调地址
    pub redirect_url: String,
    /// 自动授予管理员权限的 GitHub 用户 ID
    pub admin_github_id: Option<u64>,
}

/// 受信任数据 worker 使用的凭据
#[derive(Clone)]
pub struct WorkerConfig {
    /// worker 调用内部写入接口时使用的 bearer token
    pub token: String,
}

/// 新闻视频和 RSS 接口的客户端限流及上游刷新预算
#[derive(Clone)]
pub struct PublicRateLimitConfig {
    /// 允许提供真实客户端转发链的反向代理 IP
    pub trusted_proxy_ips: Vec<IpAddr>,
    /// 视频详情接口每分钟补充的客户端令牌数
    pub video_requests_per_minute: u32,
    /// 视频详情接口允许的客户端突发请求数
    pub video_burst: u32,
    /// RSS 接口每分钟补充的客户端令牌数
    pub rss_requests_per_minute: u32,
    /// RSS 接口允许的客户端突发请求数
    pub rss_burst: u32,
    /// 单次 RSS 请求最多触发的米游社签名刷新数
    pub rss_mys_refresh_limit: u32,
}

impl Config {
    /// 从环境变量加载并校验完整后端配置
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:7040")
                .parse()
                .context("BIND_ADDR must be a socket address")?,

            asset_base_url: required("ASSET_BASE_URL")?.trim_end_matches('/').to_owned(),
            mys_cookie: optional("MIYOUSHE_COOKIE"),

            database: DbOptions {
                pg_host: env_or("POSTGRES_HOST", "127.0.0.1"),
                pg_port: env_or("POSTGRES_PORT", "5432"),
                pg_user: required("POSTGRES_USER")?,
                pg_password: required("POSTGRES_PASSWORD")?,
                pg_database: env_or("POSTGRES_DB", "Akasha"),
            },

            auth: AuthConfig {
                jwt_secret: required_secret("JWT_SECRET")?,
                token_hash_secret: required_secret("TOKEN_HASH_SECRET")?,
            },

            github: GithubConfig {
                client_id: required("GITHUB_CLIENT_ID")?,
                client_secret: required("GITHUB_CLIENT_SECRET")?,
                redirect_url: required("GITHUB_OAUTH_REDIRECT_URL")?,
                admin_github_id: env::var("ADMIN_GITHUB_ID")
                    .ok()
                    .map(|value| value.parse())
                    .transpose()
                    .context("ADMIN_GITHUB_ID must be an unsigned integer")?,
            },

            worker: WorkerConfig {
                token: required_secret("WORKER_TOKEN")?,
            },

            public_rate_limits: PublicRateLimitConfig {
                trusted_proxy_ips: ip_address_list("RATE_LIMIT_TRUSTED_PROXY_IPS")?,
                video_requests_per_minute: positive_u32("NEWS_VIDEO_RATE_LIMIT_PER_MINUTE", 30)?,
                video_burst: positive_u32("NEWS_VIDEO_RATE_LIMIT_BURST", 10)?,
                rss_requests_per_minute: positive_u32("NEWS_RSS_RATE_LIMIT_PER_MINUTE", 12)?,
                rss_burst: positive_u32("NEWS_RSS_RATE_LIMIT_BURST", 3)?,
                rss_mys_refresh_limit: unsigned_u32("NEWS_RSS_MYS_REFRESH_LIMIT", 10)?,
            },
        })
    }
}

impl GithubConfig {
    /// 判断浏览器认证 Cookie 是否只应通过 HTTPS 发送
    pub fn uses_secure_cookies(&self) -> bool {
        self.redirect_url.starts_with("https://")
    }
}

/// 读取必需环境变量，并在缺失时提供上下文错误
fn required(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("missing required environment variable {key}"))
}

/// 读取至少包含 32 字节的认证密钥
fn required_secret(key: &str) -> Result<String> {
    const MIN_SECRET_LENGTH: usize = 32;

    let value = required(key)?;
    if value.len() < MIN_SECRET_LENGTH {
        bail!("{key} must contain at least {MIN_SECRET_LENGTH} bytes");
    }
    Ok(value)
}

/// 读取可选环境变量，空字符串视为未配置
fn optional(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// 读取环境变量，缺失时返回约定的默认值
fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// 读取大于零的 u32 配置并提供明确错误上下文
fn positive_u32(key: &str, default: u32) -> Result<u32> {
    let value = unsigned_u32(key, default)?;
    if value == 0 {
        bail!("{key} must be greater than zero");
    }
    Ok(value)
}

/// 读取允许为零的 u32 配置并提供明确错误上下文
fn unsigned_u32(key: &str, default: u32) -> Result<u32> {
    env_or(key, &default.to_string())
        .parse::<u32>()
        .with_context(|| format!("{key} must be an unsigned integer"))
}

/// 读取逗号分隔的 IP 地址列表，空值表示不信任任何代理
fn ip_address_list(key: &str) -> Result<Vec<IpAddr>> {
    let Some(value) = optional(key) else {
        return Ok(Vec::new());
    };

    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<IpAddr>()
                .with_context(|| format!("{key} contains an invalid IP address: {value}"))
        })
        .collect()
}
