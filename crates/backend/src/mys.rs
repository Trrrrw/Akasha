use std::{collections::HashMap, sync::Arc};

use akasha_mys::{MysClient, MysError, MysGame, MysVideoUrl};
use chrono::{Duration, Utc};
use tokio::sync::RwLock;

/// 在临时签名正式过期前提前刷新，避免视频刚开始播放就失效
const CACHE_REFRESH_MARGIN: Duration = Duration::seconds(30);

/// 为米游社视频按文章 ID 获取并缓存播放地址
#[derive(Clone)]
pub struct MysVideoService {
    client: Option<MysClient>,
    cache: Arc<RwLock<HashMap<VideoCacheKey, MysVideoUrl>>>,
}

/// 米游社视频地址的缓存键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VideoCacheKey {
    game_id: String,
    news_id: String,
}

/// 带刷新预算的视频地址解析结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MysVideoUrlResolution {
    /// 已从有效缓存或上游取得播放地址
    Available(String),
    /// 上游详情中没有可用视频
    NotFound,
    /// 缓存未命中且本次请求的上游刷新预算已耗尽
    RefreshBudgetExhausted,
}

impl MysVideoService {
    /// 使用共享 HTTP 客户端和可选 Cookie 创建视频服务
    pub fn new(http_client: reqwest::Client, cookie: Option<&str>) -> Result<Self, MysError> {
        let client = cookie
            .map(|cookie| MysClient::with_http_client(http_client, cookie))
            .transpose()?;

        Ok(Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 获取指定游戏和新闻的视频播放地址
    pub async fn resolve_video_url(
        &self,
        game_id: &str,
        news_id: &str,
    ) -> Result<Option<String>, MysError> {
        let resolution = self.resolve_video_url_inner(game_id, news_id, None).await?;

        match resolution {
            MysVideoUrlResolution::Available(url) => Ok(Some(url)),
            MysVideoUrlResolution::NotFound => Ok(None),
            MysVideoUrlResolution::RefreshBudgetExhausted => {
                unreachable!("无限制的视频地址解析不应耗尽刷新预算")
            }
        }
    }

    /// 在请求内预算范围内获取视频地址，缓存命中不消耗预算
    pub(crate) async fn resolve_video_url_with_refresh_budget(
        &self,
        game_id: &str,
        news_id: &str,
        remaining_refreshes: &mut u32,
    ) -> Result<MysVideoUrlResolution, MysError> {
        self.resolve_video_url_inner(game_id, news_id, Some(remaining_refreshes))
            .await
    }

    /// 复用缓存并在必要时按预算访问米游社详情接口
    async fn resolve_video_url_inner(
        &self,
        game_id: &str,
        news_id: &str,
        remaining_refreshes: Option<&mut u32>,
    ) -> Result<MysVideoUrlResolution, MysError> {
        let game = MysGame::from_game_id(game_id)?;
        let cache_key = VideoCacheKey {
            game_id: game_id.to_owned(),
            news_id: news_id.to_owned(),
        };

        // 先读取该新闻自己的缓存，不能把一个视频的签名用于其他视频
        if let Some(video) = self.cache.read().await.get(&cache_key)
            && is_cache_valid(video)
        {
            return Ok(MysVideoUrlResolution::Available(video.url.clone()));
        }

        // 只有缓存未命中且确实允许访问上游时才消耗一次刷新预算
        if remaining_refreshes.as_deref() == Some(&0) {
            return Ok(MysVideoUrlResolution::RefreshBudgetExhausted);
        }
        // 缓存失效后，按当前文章 ID 请求米游社详情接口
        let client = self
            .client
            .as_ref()
            .ok_or(MysError::MissingCookieValue("MIYOUSHE_COOKIE"))?;
        if let Some(remaining_refreshes) = remaining_refreshes {
            *remaining_refreshes -= 1;
        }
        let Some(video) = client.get_video_url(game, news_id).await? else {
            return Ok(MysVideoUrlResolution::NotFound);
        };

        // 只缓存这条新闻对应的签名地址
        let video_url = video.url.clone();
        self.cache.write().await.insert(cache_key, video);

        Ok(MysVideoUrlResolution::Available(video_url))
    }
}

/// 判断缓存中的视频地址是否值得继续复用
fn is_cache_valid(video: &MysVideoUrl) -> bool {
    video
        .expires_at
        .is_none_or(|expires_at| expires_at > Utc::now() + CACHE_REFRESH_MARGIN)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use akasha_mys::MysVideoUrl;

    use super::{MysVideoService, MysVideoUrlResolution, VideoCacheKey};

    /// 同一新闻在签名有效期内复用缓存地址
    #[tokio::test]
    async fn reuses_the_cached_url_for_the_same_news() {
        let service = MysVideoService::new(reqwest::Client::new(), None).expect("创建视频服务");
        service.cache.write().await.insert(
            VideoCacheKey {
                game_id: "ys".to_owned(),
                news_id: "news-1".to_owned(),
            },
            MysVideoUrl {
                url: "https://video.example/news-1.mp4?auth_key=test".to_owned(),
                expires_at: Some(Utc::now() + Duration::minutes(5)),
            },
        );

        let video_url = service
            .resolve_video_url("ys", "news-1")
            .await
            .expect("读取缓存视频地址");

        assert_eq!(
            video_url.as_deref(),
            Some("https://video.example/news-1.mp4?auth_key=test")
        );
    }

    /// 不同新闻之间不能错误复用完整视频地址
    #[tokio::test]
    async fn does_not_share_a_cached_url_between_news() {
        let service = MysVideoService::new(reqwest::Client::new(), None).expect("创建视频服务");
        service.cache.write().await.insert(
            VideoCacheKey {
                game_id: "ys".to_owned(),
                news_id: "news-1".to_owned(),
            },
            MysVideoUrl {
                url: "https://video.example/news-1.mp4?auth_key=test".to_owned(),
                expires_at: Some(Utc::now() + Duration::minutes(5)),
            },
        );

        let error = service
            .resolve_video_url("ys", "news-2")
            .await
            .expect_err("未配置 Cookie 时不能为新视频获取地址");

        assert!(error.to_string().contains("MIYOUSHE_COOKIE"));
    }

    /// 零刷新预算仍允许复用有效缓存
    #[tokio::test]
    async fn cached_video_does_not_consume_refresh_budget() {
        let service = MysVideoService::new(reqwest::Client::new(), None).expect("创建视频服务");
        service.cache.write().await.insert(
            VideoCacheKey {
                game_id: "ys".to_owned(),
                news_id: "news-1".to_owned(),
            },
            MysVideoUrl {
                url: "https://video.example/news-1.mp4?auth_key=test".to_owned(),
                expires_at: Some(Utc::now() + Duration::minutes(5)),
            },
        );
        let mut remaining_refreshes = 0;

        let resolution = service
            .resolve_video_url_with_refresh_budget("ys", "news-1", &mut remaining_refreshes)
            .await
            .expect("应读取缓存视频地址");

        assert!(matches!(resolution, MysVideoUrlResolution::Available(_)));
        assert_eq!(remaining_refreshes, 0);
    }

    /// 无签名静态视频地址在缓存中应长期复用
    #[tokio::test]
    async fn reuses_unsigned_video_without_cookie() {
        let service = MysVideoService::new(reqwest::Client::new(), None).expect("创建视频服务");
        service.cache.write().await.insert(
            VideoCacheKey {
                game_id: "ys".to_owned(),
                news_id: "news-static".to_owned(),
            },
            MysVideoUrl {
                url: "https://vod-static.miyoushe.com/news-static.mp4".to_owned(),
                expires_at: None,
            },
        );

        let video_url = service
            .resolve_video_url("ys", "news-static")
            .await
            .expect("读取静态视频地址");

        assert_eq!(
            video_url.as_deref(),
            Some("https://vod-static.miyoushe.com/news-static.mp4")
        );
    }

    /// 缓存未命中时不得突破请求内刷新预算
    #[tokio::test]
    async fn stops_before_upstream_when_refresh_budget_is_exhausted() {
        let service = MysVideoService::new(reqwest::Client::new(), None).expect("创建视频服务");
        let mut remaining_refreshes = 0;

        let resolution = service
            .resolve_video_url_with_refresh_budget("ys", "news-1", &mut remaining_refreshes)
            .await
            .expect("预算耗尽不应访问上游");

        assert_eq!(resolution, MysVideoUrlResolution::RefreshBudgetExhausted);
    }
}
