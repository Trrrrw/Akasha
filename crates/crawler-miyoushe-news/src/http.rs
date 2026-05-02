use reqwest::{
    IntoUrl, StatusCode,
    header::{
        ACCEPT, ACCEPT_LANGUAGE, CONNECTION, COOKIE, HeaderMap, HeaderValue, ORIGIN, REFERER,
        USER_AGENT,
    },
};
use serde::de::DeserializeOwned;
use std::time::Duration;

use crawler_core::warn;

const TOO_MANY_REQUESTS_MAX_RETRIES: u32 = 5;
const TOO_MANY_REQUESTS_RETRY_DELAY: Duration = Duration::from_secs(10 * 60);

pub async fn get<U, T>(url: U) -> anyhow::Result<T>
where
    U: IntoUrl,
    T: DeserializeOwned,
{
    let client = reqwest::Client::new();
    let url = url.into_url()?;

    for attempt in 1..=TOO_MANY_REQUESTS_MAX_RETRIES {
        let response = client.get(url.clone()).headers(headers()?).send().await?;

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            if attempt == TOO_MANY_REQUESTS_MAX_RETRIES {
                response.error_for_status()?;
            }

            warn!(
                url = %url,
                attempt = attempt,
                max_retries = TOO_MANY_REQUESTS_MAX_RETRIES,
                retry_after_secs = TOO_MANY_REQUESTS_RETRY_DELAY.as_secs(),
                "米游社请求触发 429 限流，等待后重试"
            );
            tokio::time::sleep(TOO_MANY_REQUESTS_RETRY_DELAY).await;
            continue;
        }

        let text = response.error_for_status()?.text().await?;

        let data = serde_json::from_str::<T>(&text)
            .map_err(|err| anyhow::anyhow!("JSON 解析失败: {err}; 响应内容: {text}"))?;

        return Ok(data);
    }

    unreachable!("retry loop either returns data or propagates the final 429 error")
}

fn headers() -> anyhow::Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
        ),
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/plain, */*"),
    );
    headers.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static("zh-CN,zh;q=0.9,zh-TW;q=0.8,zh-HK;q=0.7,en-US;q=0.6,en;q=0.5"),
    );
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://www.miyoushe.com/"),
    );
    headers.insert(ORIGIN, HeaderValue::from_static("https://www.miyoushe.com"));
    let cookie = std::env::var("MIYOUSHE_COOKIE")
        .map_err(|_| anyhow::anyhow!("缺少必需的环境变量 MIYOUSHE_COOKIE"))?;
    let cookie = HeaderValue::from_str(&cookie)
        .map_err(|err| anyhow::anyhow!("MIYOUSHE_COOKIE 不是合法的请求头值: {err}"))?;
    headers.insert(COOKIE, cookie);
    headers.insert("x-rpc-client_type", HeaderValue::from_static("4"));
    headers.insert("x-rpc-app_version", HeaderValue::from_static("2.102.0"));
    headers.insert(
        "x-rpc-device_id",
        HeaderValue::from_static("b95924a6-12cd-4f37-95c4-b5af8fea295a"),
    );
    headers.insert("x-rpc-device_fp", HeaderValue::from_static("38d8168c22ac8"));
    headers.insert("sec-gpc", HeaderValue::from_static("1"));
    headers.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
    headers.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
    headers.insert("sec-fetch-site", HeaderValue::from_static("same-site"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));

    Ok(headers)
}
