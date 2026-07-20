use crate::{config::GitHubConfig, http::error::AppError};
use reqwest::header;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(super) struct Token {
    pub access_token: String,
}
#[derive(Deserialize)]
pub(super) struct User {
    pub id: u64,
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
}
#[derive(Serialize)]
struct TokenRequest<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    code: &'a str,
    redirect_uri: &'a str,
}
pub(super) async fn exchange(
    client: &reqwest::Client,
    config: &GitHubConfig,
    code: &str,
) -> Result<Token, AppError> {
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header(header::ACCEPT, "application/json")
        .form(&TokenRequest {
            client_id: &config.client_id,
            client_secret: &config.client_secret,
            code,
            redirect_uri: &config.redirect_url,
        })
        .send()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    if !response.status().is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "github token endpoint returned {}",
            response.status()
        )));
    }
    response
        .json()
        .await
        .map_err(|e| AppError::Internal(e.into()))
}
pub(super) async fn user(client: &reqwest::Client, token: &str) -> Result<User, AppError> {
    let response = client
        .get("https://api.github.com/user")
        .header(header::ACCEPT, "application/vnd.github+json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    if !response.status().is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "github user endpoint returned {}",
            response.status()
        )));
    }
    response
        .json()
        .await
        .map_err(|e| AppError::Internal(e.into()))
}
