use reqwest::{IntoUrl, header::USER_AGENT};
use serde::{Serialize, de::DeserializeOwned};

pub async fn get<U, P, T>(url: U, params: &P) -> anyhow::Result<T>
where
    U: IntoUrl,
    P: Serialize + ?Sized,
    T: DeserializeOwned,
{
    let client = reqwest::Client::new();

    let text = client
        .get(url)
        .query(params)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
        )
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let data = serde_json::from_str::<T>(&text)
        .map_err(|err| anyhow::anyhow!("json decode failed: {err}; body: {text}"))?;

    Ok(data)
}
