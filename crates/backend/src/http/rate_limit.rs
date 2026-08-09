use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::http::HeaderMap;

use crate::config::PublicRateLimitConfig;

const SECONDS_PER_MINUTE: f64 = 60.0;
const CLIENT_CLEANUP_INTERVAL: Duration = Duration::from_secs(60);
const MIN_CLIENT_RETENTION: Duration = Duration::from_secs(120);
const MAX_TRACKED_CLIENTS: usize = 65_536;

/// 新闻公开接口使用的独立客户端限流器
#[derive(Clone)]
pub(crate) struct PublicRateLimiters {
    video: ClientRateLimiter,
    rss: ClientRateLimiter,
    trusted_proxy_ips: Arc<HashSet<IpAddr>>,
}

impl PublicRateLimiters {
    /// 根据运行时配置创建视频详情和 RSS 限流器
    pub(crate) fn new(config: &PublicRateLimitConfig) -> Self {
        Self {
            video: ClientRateLimiter::new(config.video_requests_per_minute, config.video_burst),
            rss: ClientRateLimiter::new(config.rss_requests_per_minute, config.rss_burst),
            trusted_proxy_ips: Arc::new(config.trusted_proxy_ips.iter().copied().collect()),
        }
    }

    /// 从直连地址和可信代理转发链解析限流使用的客户端 IP
    pub(crate) fn client_ip(&self, headers: &HeaderMap, peer_address: SocketAddr) -> IpAddr {
        resolve_client_ip(headers, peer_address.ip(), &self.trusted_proxy_ips)
    }

    /// 检查客户端是否仍可请求视频详情接口
    pub(crate) fn check_video(&self, client_ip: IpAddr) -> Result<(), RateLimitExceeded> {
        self.video.check(client_ip)
    }

    /// 检查客户端是否仍可请求 RSS 接口
    pub(crate) fn check_rss(&self, client_ip: IpAddr) -> Result<(), RateLimitExceeded> {
        self.rss.check(client_ip)
    }
}

/// 仅在直连来源可信时从右向左解析 X-Forwarded-For 代理链
fn resolve_client_ip(
    headers: &HeaderMap,
    peer_ip: IpAddr,
    trusted_proxy_ips: &HashSet<IpAddr>,
) -> IpAddr {
    if !trusted_proxy_ips.contains(&peer_ip) {
        return peer_ip;
    }

    let Some(forwarded_for) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    else {
        return peer_ip;
    };
    let forwarded_ips = forwarded_for
        .split(',')
        .map(str::trim)
        .map(str::parse::<IpAddr>)
        .collect::<Result<Vec<_>, _>>();
    let Ok(forwarded_ips) = forwarded_ips else {
        return peer_ip;
    };

    let mut client_ip = peer_ip;
    for forwarded_ip in forwarded_ips.into_iter().rev() {
        if !trusted_proxy_ips.contains(&client_ip) {
            break;
        }
        client_ip = forwarded_ip;
    }

    client_ip
}

/// 一次客户端限流拒绝及建议重试时间
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RateLimitExceeded {
    pub(crate) retry_after_seconds: u64,
}

/// 按客户端 IP 隔离的内存令牌桶限流器
#[derive(Clone)]
struct ClientRateLimiter {
    refill_per_second: f64,
    capacity: f64,
    client_retention: Duration,
    state: Arc<Mutex<RateLimitState>>,
}

impl ClientRateLimiter {
    /// 使用每分钟补充数量和突发容量创建限流器
    fn new(requests_per_minute: u32, burst: u32) -> Self {
        let refill_per_second = f64::from(requests_per_minute) / SECONDS_PER_MINUTE;
        let capacity = f64::from(burst);
        let refill_duration = Duration::from_secs_f64(capacity / refill_per_second);

        Self {
            refill_per_second,
            capacity,
            client_retention: MIN_CLIENT_RETENTION.max(refill_duration.saturating_mul(2)),
            state: Arc::new(Mutex::new(RateLimitState::new())),
        }
    }

    /// 消耗一个客户端令牌，令牌不足时返回最短等待时间
    fn check(&self, client_ip: IpAddr) -> Result<(), RateLimitExceeded> {
        self.check_at(client_ip, Instant::now())
    }

    /// 在指定时刻执行检查，便于稳定测试令牌补充行为
    fn check_at(&self, client_ip: IpAddr, now: Instant) -> Result<(), RateLimitExceeded> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // 定期清理长时间未访问的客户端，限制内存随来源地址增长
        if now.duration_since(state.last_cleanup) >= CLIENT_CLEANUP_INTERVAL {
            state
                .clients
                .retain(|_, bucket| now.duration_since(bucket.last_seen) <= self.client_retention);
            state.last_cleanup = now;
        }

        if !state.clients.contains_key(&client_ip) && state.clients.len() >= MAX_TRACKED_CLIENTS {
            return Err(RateLimitExceeded {
                retry_after_seconds: CLIENT_CLEANUP_INTERVAL.as_secs(),
            });
        }

        let bucket = state
            .clients
            .entry(client_ip)
            .or_insert_with(|| ClientBucket::new(self.capacity, now));
        bucket.refill(now, self.refill_per_second, self.capacity);

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            return Ok(());
        }

        let missing_tokens = 1.0 - bucket.tokens;
        let retry_after_seconds = (missing_tokens / self.refill_per_second).ceil().max(1.0) as u64;
        Err(RateLimitExceeded {
            retry_after_seconds,
        })
    }
}

/// 全部客户端令牌桶和最近一次清理时间
struct RateLimitState {
    clients: HashMap<IpAddr, ClientBucket>,
    last_cleanup: Instant,
}

impl RateLimitState {
    /// 创建空的客户端限流状态
    fn new() -> Self {
        Self {
            clients: HashMap::new(),
            last_cleanup: Instant::now(),
        }
    }
}

/// 单个客户端当前持有的令牌状态
struct ClientBucket {
    tokens: f64,
    last_refill: Instant,
    last_seen: Instant,
}

impl ClientBucket {
    /// 使用完整突发容量初始化客户端令牌桶
    fn new(capacity: f64, now: Instant) -> Self {
        Self {
            tokens: capacity,
            last_refill: now,
            last_seen: now,
        }
    }

    /// 根据经过时间补充令牌并记录最近访问时间
    fn refill(&mut self, now: Instant, refill_per_second: f64, capacity: f64) {
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * refill_per_second).min(capacity);
        self.last_refill = now;
        self.last_seen = now;
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        net::{IpAddr, SocketAddr},
        time::Instant,
    };

    use axum::http::{HeaderMap, HeaderValue};

    use super::{ClientRateLimiter, resolve_client_ip};

    /// 突发令牌耗尽后拒绝请求并给出重试时间
    #[test]
    fn rejects_requests_after_burst_is_exhausted() {
        let limiter = ClientRateLimiter::new(60, 2);
        let client_ip = IpAddr::from([192, 0, 2, 1]);
        let now = Instant::now();

        assert!(limiter.check_at(client_ip, now).is_ok());
        assert!(limiter.check_at(client_ip, now).is_ok());
        let exceeded = limiter
            .check_at(client_ip, now)
            .expect_err("第三个请求应被拒绝");

        assert_eq!(exceeded.retry_after_seconds, 1);
    }

    /// 令牌会随时间补充且不同客户端互不影响
    #[test]
    fn refills_tokens_and_separates_clients() {
        let limiter = ClientRateLimiter::new(60, 1);
        let first_client = IpAddr::from([192, 0, 2, 1]);
        let second_client = IpAddr::from([192, 0, 2, 2]);
        let now = Instant::now();

        assert!(limiter.check_at(first_client, now).is_ok());
        assert!(limiter.check_at(first_client, now).is_err());
        assert!(limiter.check_at(second_client, now).is_ok());
        assert!(
            limiter
                .check_at(first_client, now + std::time::Duration::from_secs(1))
                .is_ok()
        );
    }

    /// 未受信任来源提供的转发头不能伪造限流身份
    #[test]
    fn ignores_forwarded_header_from_untrusted_peer() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("198.51.100.10"));
        let peer_ip = IpAddr::from([192, 0, 2, 10]);

        assert_eq!(
            resolve_client_ip(&headers, peer_ip, &HashSet::new()),
            peer_ip
        );
    }

    /// 可信代理链从右向左跳过代理并返回首个非代理地址
    #[test]
    fn resolves_client_through_trusted_proxy_chain() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.8, 10.0.0.2"),
        );
        let peer_address = SocketAddr::from(([10, 0, 0, 1], 7040));
        let trusted_proxy_ips =
            HashSet::from([IpAddr::from([10, 0, 0, 1]), IpAddr::from([10, 0, 0, 2])]);

        assert_eq!(
            resolve_client_ip(&headers, peer_address.ip(), &trusted_proxy_ips),
            IpAddr::from([203, 0, 113, 8])
        );
    }
}
