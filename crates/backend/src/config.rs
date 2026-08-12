use std::{
    env, fs,
    io::ErrorKind,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use akasha_db::DbOptions;
use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

const DEFAULT_CONFIG_FILE: &str = "config/backend.toml";

/// 从配置文件和环境变量读取的运行时配置
#[derive(Clone)]
pub struct Config {
    /// 日志级别过滤器
    pub log_level: String,
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
    /// 审计日志保留天数
    pub audit_log_retention_days: u32,
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

/// 配置文件的顶层结构，所有字段都可由环境变量回退或覆盖
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct FileConfig {
    server: FileServerConfig,
    database: FileDatabaseConfig,
    auth: FileAuthConfig,
    github: FileGithubConfig,
    worker: FileWorkerConfig,
    mys: FileMysConfig,
    rate_limits: FileRateLimitConfig,
    audit: FileAuditConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct FileServerConfig {
    log_level: Option<String>,
    bind_addr: Option<String>,
    asset_base_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct FileDatabaseConfig {
    host: Option<String>,
    port: Option<String>,
    user: Option<String>,
    password: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct FileAuthConfig {
    jwt_secret: Option<String>,
    token_hash_secret: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct FileGithubConfig {
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_url: Option<String>,
    admin_github_id: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct FileWorkerConfig {
    token: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct FileMysConfig {
    cookie: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct FileRateLimitConfig {
    trusted_proxy_ips: Option<Vec<String>>,
    video_requests_per_minute: Option<u32>,
    video_burst: Option<u32>,
    rss_requests_per_minute: Option<u32>,
    rss_burst: Option<u32>,
    rss_mys_refresh_limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct FileAuditConfig {
    retention_days: Option<u32>,
}

impl FileConfig {
    /// 读取默认路径或环境变量指定的配置文件
    fn load() -> Result<Self> {
        let configured_path = env::var("AKASHA_CONFIG_FILE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from);
        let path = configured_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));

        match fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content)
                .with_context(|| format!("failed to parse config file {}", path.display())),
            Err(error) if error.kind() == ErrorKind::NotFound && configured_path.is_none() => {
                Ok(Self::default())
            }
            Err(error) => {
                Err(error).with_context(|| format!("failed to read config file {}", path.display()))
            }
        }
    }
}

impl Config {
    /// 从配置文件和环境变量加载并校验完整后端配置
    pub fn load() -> Result<Self> {
        let file = FileConfig::load()?;

        Ok(Self {
            log_level: string_value("LOG_LEVEL", file.server.log_level.as_deref(), "info"),
            bind_addr: string_value(
                "BIND_ADDR",
                file.server.bind_addr.as_deref(),
                "0.0.0.0:7040",
            )
            .parse()
            .context("BIND_ADDR must be a socket address")?,

            asset_base_url: required_value(
                "ASSET_BASE_URL",
                file.server.asset_base_url.as_deref(),
            )?
            .trim_end_matches('/')
            .to_owned(),
            mys_cookie: optional_value("MIYOUSHE_COOKIE", file.mys.cookie.as_deref()),

            database: DbOptions {
                pg_host: string_value("POSTGRES_HOST", file.database.host.as_deref(), "127.0.0.1"),
                pg_port: string_value("POSTGRES_PORT", file.database.port.as_deref(), "5432"),
                pg_user: required_value("POSTGRES_USER", file.database.user.as_deref())?,
                pg_password: required_value(
                    "POSTGRES_PASSWORD",
                    file.database.password.as_deref(),
                )?,
                pg_database: string_value("POSTGRES_DB", file.database.name.as_deref(), "Akasha"),
            },

            auth: AuthConfig {
                jwt_secret: required_secret("JWT_SECRET", file.auth.jwt_secret.as_deref())?,
                token_hash_secret: required_secret(
                    "TOKEN_HASH_SECRET",
                    file.auth.token_hash_secret.as_deref(),
                )?,
            },

            github: GithubConfig {
                client_id: required_value("GITHUB_CLIENT_ID", file.github.client_id.as_deref())?,
                client_secret: required_value(
                    "GITHUB_CLIENT_SECRET",
                    file.github.client_secret.as_deref(),
                )?,
                redirect_url: required_value(
                    "GITHUB_OAUTH_REDIRECT_URL",
                    file.github.redirect_url.as_deref(),
                )?,
                admin_github_id: optional_u64("ADMIN_GITHUB_ID", file.github.admin_github_id)?,
            },

            worker: WorkerConfig {
                token: required_secret("WORKER_TOKEN", file.worker.token.as_deref())?,
            },

            public_rate_limits: PublicRateLimitConfig {
                trusted_proxy_ips: ip_address_list(
                    "RATE_LIMIT_TRUSTED_PROXY_IPS",
                    file.rate_limits.trusted_proxy_ips.as_deref(),
                )?,
                video_requests_per_minute: positive_u32(
                    "NEWS_VIDEO_RATE_LIMIT_PER_MINUTE",
                    file.rate_limits.video_requests_per_minute,
                    30,
                )?,
                video_burst: positive_u32(
                    "NEWS_VIDEO_RATE_LIMIT_BURST",
                    file.rate_limits.video_burst,
                    10,
                )?,
                rss_requests_per_minute: positive_u32(
                    "NEWS_RSS_RATE_LIMIT_PER_MINUTE",
                    file.rate_limits.rss_requests_per_minute,
                    12,
                )?,
                rss_burst: positive_u32(
                    "NEWS_RSS_RATE_LIMIT_BURST",
                    file.rate_limits.rss_burst,
                    3,
                )?,
                rss_mys_refresh_limit: unsigned_u32(
                    "NEWS_RSS_MYS_REFRESH_LIMIT",
                    file.rate_limits.rss_mys_refresh_limit,
                    10,
                )?,
            },

            audit_log_retention_days: positive_u32(
                "AUDIT_LOG_RETENTION_DAYS",
                file.audit.retention_days,
                180,
            )?,
        })
    }
}

impl GithubConfig {
    /// 判断浏览器认证 Cookie 是否只应通过 HTTPS 发送
    pub fn uses_secure_cookies(&self) -> bool {
        self.redirect_url.starts_with("https://")
    }
}

/// 返回环境变量、配置文件或默认值中的第一个非空字符串
fn string_value(key: &str, file_value: Option<&str>, default: &str) -> String {
    env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| file_value.map(ToOwned::to_owned))
        .unwrap_or_else(|| default.to_owned())
}

/// 返回环境变量或配置文件中的必需字符串
fn required_value(key: &str, file_value: Option<&str>) -> Result<String> {
    env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            file_value
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned)
        })
        .ok_or_else(|| anyhow!("missing required configuration value {key}"))
}

/// 读取至少包含 32 字节的认证密钥
fn required_secret(key: &str, file_value: Option<&str>) -> Result<String> {
    const MIN_SECRET_LENGTH: usize = 32;

    let value = required_value(key, file_value)?;
    if value.len() < MIN_SECRET_LENGTH {
        bail!("{key} must contain at least {MIN_SECRET_LENGTH} bytes");
    }
    Ok(value)
}

/// 返回可选的环境变量或配置文件字符串，空字符串视为未配置
fn optional_value(key: &str, file_value: Option<&str>) -> Option<String> {
    env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            file_value
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned)
        })
}

/// 读取可选的管理员用户 ID
fn optional_u64(key: &str, file_value: Option<u64>) -> Result<Option<u64>> {
    let Some(value) = env::var(key).ok().filter(|value| !value.trim().is_empty()) else {
        return Ok(file_value);
    };

    value
        .parse::<u64>()
        .map(Some)
        .with_context(|| format!("{key} must be an unsigned integer"))
}

/// 读取大于零的 u32 配置并提供明确错误上下文
fn positive_u32(key: &str, file_value: Option<u32>, default: u32) -> Result<u32> {
    let value = unsigned_u32(key, file_value, default)?;
    if value == 0 {
        bail!("{key} must be greater than zero");
    }
    Ok(value)
}

/// 读取允许为零的 u32 配置并提供明确错误上下文
fn unsigned_u32(key: &str, file_value: Option<u32>, default: u32) -> Result<u32> {
    let value = env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| file_value.map(|value| value.to_string()))
        .unwrap_or_else(|| default.to_string());

    value
        .parse::<u32>()
        .with_context(|| format!("{key} must be an unsigned integer"))
}

/// 读取逗号分隔的环境变量或配置文件 IP 地址列表
fn ip_address_list(key: &str, file_values: Option<&[String]>) -> Result<Vec<IpAddr>> {
    let values = if let Ok(value) = env::var(key) {
        value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    } else {
        file_values.unwrap_or_default().to_vec()
    };

    values
        .iter()
        .map(|value| {
            value
                .parse::<IpAddr>()
                .with_context(|| format!("{key} contains an invalid IP address: {value}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::FileConfig;

    #[test]
    fn example_config_is_valid_toml() {
        let config: FileConfig =
            toml::from_str(include_str!("../../../config/backend.toml.example"))
                .expect("backend.toml.example must be valid TOML");

        assert_eq!(config.server.bind_addr.as_deref(), Some("0.0.0.0:7040"));
        assert_eq!(config.database.name.as_deref(), Some("Akasha"));
        assert_eq!(config.audit.retention_days, Some(180));
    }
}
