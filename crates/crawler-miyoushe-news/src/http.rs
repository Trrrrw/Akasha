use reqwest::{
    IntoUrl,
    header::{
        ACCEPT, ACCEPT_LANGUAGE, CONNECTION, COOKIE, HeaderMap, HeaderValue, ORIGIN, REFERER,
        USER_AGENT,
    },
};
use serde::de::DeserializeOwned;

pub async fn get<U, T>(url: U) -> anyhow::Result<T>
where
    U: IntoUrl,
    T: DeserializeOwned,
{
    let client = reqwest::Client::new();

    let text = client
        .get(url)
        .headers(headers()?)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let data = serde_json::from_str::<T>(&text)
        .map_err(|err| anyhow::anyhow!("json decode failed: {err}; body: {text}"))?;

    Ok(data)
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
        .map_err(|_| anyhow::anyhow!("missing required environment variable MIYOUSHE_COOKIE"))?;
    let cookie = HeaderValue::from_str(&cookie)
        .map_err(|err| anyhow::anyhow!("invalid MIYOUSHE_COOKIE header value: {err}"))?;
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
