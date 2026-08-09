use std::{sync::Arc, time::Duration};

use akasha_application::ApplicationServices;
use akasha_db::Db;
use anyhow::Result;

use crate::{Config, http::rate_limit::PublicRateLimiters, mys::MysVideoService};

/// 暴露给 HTTP handler 和请求提取器的共享依赖
#[derive(Clone)]
pub struct AppState {
    config: Arc<Config>,
    application: ApplicationServices<Db>,
    http_client: reqwest::Client,
    mys_video_service: MysVideoService,
    public_rate_limiters: PublicRateLimiters,
}

impl AppState {
    /// 连接基础设施依赖并构建应用服务门面
    pub async fn new(config: Config) -> Result<Self> {
        let db = Db::init(config.database.clone()).await?;
        let application = ApplicationServices::new(db);
        let http_client = reqwest::Client::builder()
            .user_agent("akasha-backend")
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()?;
        let mys_video_service =
            MysVideoService::new(http_client.clone(), config.mys_cookie.as_deref())?;
        let public_rate_limiters = PublicRateLimiters::new(&config.public_rate_limits);

        Ok(Self {
            config: Arc::new(config),
            application,
            http_client,
            mys_video_service,
            public_rate_limiters,
        })
    }

    /// 返回不可变的 HTTP 和运行时配置
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 返回 HTTP handler 和请求提取器使用的应用服务
    pub fn application(&self) -> &ApplicationServices<Db> {
        &self.application
    }

    /// 返回外部 API 集成共用的 HTTP 客户端
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }

    /// 返回米游社视频播放地址服务
    pub fn mys_video_service(&self) -> &MysVideoService {
        &self.mys_video_service
    }

    /// 返回新闻公开接口使用的客户端限流器
    pub(crate) fn public_rate_limiters(&self) -> &PublicRateLimiters {
        &self.public_rate_limiters
    }
}
