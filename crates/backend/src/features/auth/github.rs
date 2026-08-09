use crate::{config::GithubConfig, http::error::AppError};
use reqwest::header;
use serde::{Deserialize, Serialize};

/// GitHub OAuth token 接口返回的最小响应
#[derive(Deserialize)]
pub(super) struct GithubAccessTokenResponse {
    /// GitHub 签发的 OAuth access token
    pub access_token: String,
}

/// GitHub 用户接口返回的登录用户资料
#[derive(Deserialize)]
pub(super) struct GithubUserResponse {
    /// GitHub 数字用户 ID
    pub id: u64,
    /// GitHub 登录名
    pub login: String,
    /// 用户展示名称
    pub name: Option<String>,
    /// 头像地址
    pub avatar_url: Option<String>,
    /// 公开邮箱
    pub email: Option<String>,
}

/// 交换 GitHub OAuth token 时使用的请求体
#[derive(Serialize)]
struct GithubTokenRequest<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    code: &'a str,
    redirect_uri: &'a str,
}

/// 使用 OAuth 回调 code 换取 GitHub access token
pub(super) async fn exchange_code_for_token(
    http_client: &reqwest::Client,
    config: &GithubConfig,
    code: &str,
) -> Result<GithubAccessTokenResponse, AppError> {
    let response = http_client
        .post("https://github.com/login/oauth/access_token")
        .header(header::ACCEPT, "application/json")
        .form(&GithubTokenRequest {
            client_id: &config.client_id,
            client_secret: &config.client_secret,
            code,
            redirect_uri: &config.redirect_url,
        })
        .send()
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    if !response.status().is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "github token endpoint returned {}",
            response.status()
        )));
    }
    response
        .json()
        .await
        .map_err(|error| AppError::Internal(error.into()))
}

/// 获取 OAuth access token 对应的 GitHub 用户资料
pub(super) async fn fetch_authenticated_user(
    http_client: &reqwest::Client,
    token: &str,
) -> Result<GithubUserResponse, AppError> {
    let response = http_client
        .get("https://api.github.com/user")
        .header(header::ACCEPT, "application/vnd.github+json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    if !response.status().is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "github user endpoint returned {}",
            response.status()
        )));
    }
    response
        .json()
        .await
        .map_err(|error| AppError::Internal(error.into()))
}
