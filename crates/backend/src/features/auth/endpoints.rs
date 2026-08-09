use crate::{
    features::auth::{
        dto::{MeResponse, MessageResponse, TokenResponse},
        github,
        query::GithubCallbackQuery,
        token,
    },
    http::{error::AppError, response::ErrorResponse},
    state::AppState,
};
use akasha_application::auth::{GithubUserProfile, RefreshTokenMetadata};
use axum::{
    Json,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, header},
    response::{AppendHeaders, IntoResponse, Redirect},
};
use std::net::SocketAddr;

const COOKIE_PATH: &str = "/api/v1/auth";
const OAUTH_STATE_MAX_AGE_SECONDS: u32 = 10 * 60;
const REFRESH_TOKEN_MAX_AGE_SECONDS: u32 = 30 * 24 * 60 * 60;

/// 为认证路由范围格式化安全的仅 HTTP Cookie 响应头
fn build_auth_cookie(name: &str, value: &str, max_age: u32, secure: bool) -> String {
    let secure_attribute = if secure { "; Secure" } else { "" };
    format!(
        "{name}={value}; Path={COOKIE_PATH}; Max-Age={max_age}; HttpOnly; SameSite=Lax{secure_attribute}"
    )
}

/// 提取与 refresh token 记录一同持久化的请求元数据
fn refresh_token_metadata(headers: &HeaderMap, client_address: SocketAddr) -> RefreshTokenMetadata {
    RefreshTokenMetadata {
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned),
        ip_address: Some(client_address.ip().to_string()),
    }
}

/// 创建 state Cookie 并跳转至 GitHub 以启动 OAuth
#[utoipa::path(
    get,
    path = "/auth/github",
    tag = "Auth",
    summary = "开始 GitHub 登录",
    description = "写入一次性防伪 Cookie，并将浏览器重定向至 GitHub OAuth 授权页面",
    responses(
        (status = 307, description = "重定向至 GitHub OAuth 授权页面"),
        (status = 500, body = ErrorResponse)
    )
)]
pub(crate) async fn github_login(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let oauth_state = token::generate_random_token();
    let config = state.config();
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read:user&state={}",
        urlencoding::encode(&config.github.client_id),
        urlencoding::encode(&config.github.redirect_url),
        urlencoding::encode(&oauth_state)
    );
    Ok((
        AppendHeaders([(
            header::SET_COOKIE,
            build_auth_cookie(
                token::OAUTH_STATE_COOKIE,
                &oauth_state,
                OAUTH_STATE_MAX_AGE_SECONDS,
                config.github.uses_secure_cookies(),
            ),
        )]),
        Redirect::temporary(&url),
    ))
}

/// 完成 GitHub OAuth 并创建浏览器的 refresh token 会话
#[utoipa::path(
    get,
    path = "/auth/callback/github",
    tag = "Auth",
    summary = "完成 GitHub 登录",
    description = "校验 GitHub 回调和防伪 Cookie，创建刷新会话后重定向至 Scalar",
    params(GithubCallbackQuery),
    security(("oauth_state" = [])),
    responses(
        (status = 307, description = "登录成功并重定向至 Scalar"),
        (status = 401, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(crate) async fn github_callback(
    State(state): State<AppState>,
    Query(query): Query<GithubCallbackQuery>,
    ConnectInfo(client_address): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    // 先验证浏览器 state，再向 GitHub 换取用户身份
    token::validate_state(&state.config().auth, &headers, &query.state)?;
    let config = state.config();
    let github_token =
        github::exchange_code_for_token(state.http_client(), &config.github, &query.code).await?;
    let profile =
        github::fetch_authenticated_user(state.http_client(), &github_token.access_token).await?;
    let user = state
        .application()
        .upsert_github_user(GithubUserProfile {
            provider_user_id: profile.id.to_string(),
            provider_login: profile.login.clone(),
            display_name: profile.name.unwrap_or(profile.login),
            email: profile.email,
            avatar_url: profile.avatar_url,
            is_admin: config.github.admin_github_id == Some(profile.id),
        })
        .await?;
    // 持久化 refresh token 哈希，原始 token 仅写入浏览器 Cookie
    let refresh_token = token::generate_refresh_token();
    let refresh_token_hash = token::hash_sensitive_token(&config.auth, &refresh_token)?;
    state
        .application()
        .save_refresh_token(
            user.id,
            refresh_token_hash,
            refresh_token_metadata(&headers, client_address),
        )
        .await?;
    Ok((
        AppendHeaders([
            (
                header::SET_COOKIE,
                build_auth_cookie(
                    token::OAUTH_STATE_COOKIE,
                    "",
                    0,
                    config.github.uses_secure_cookies(),
                ),
            ),
            (
                header::SET_COOKIE,
                build_auth_cookie(
                    token::REFRESH_TOKEN_COOKIE,
                    &refresh_token,
                    REFRESH_TOKEN_MAX_AGE_SECONDS,
                    config.github.uses_secure_cookies(),
                ),
            ),
        ]),
        Redirect::temporary("/scalar"),
    ))
}

/// 轮换浏览器 refresh token 并返回新的短期 access token
#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "Auth",
    summary = "刷新访问令牌",
    description = "使用浏览器中的 HttpOnly 刷新令牌 Cookie 轮换会话并返回短期访问令牌",
    security(("refresh_token" = [])),
    responses(
        (status = 200, body = TokenResponse),
        (status = 401, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(crate) async fn refresh_session(
    State(state): State<AppState>,
    ConnectInfo(client_address): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let previous_refresh_token = token::read_cookie(&headers, token::REFRESH_TOKEN_COOKIE)
        .ok_or_else(|| AppError::Unauthorized("missing refresh token cookie".into()))?;
    let config = state.config();
    let next_refresh_token = token::generate_refresh_token();
    let user = state
        .application()
        .rotate_refresh_token(
            token::hash_sensitive_token(&config.auth, &previous_refresh_token)?,
            token::hash_sensitive_token(&config.auth, &next_refresh_token)?,
            refresh_token_metadata(&headers, client_address),
        )
        .await?;
    Ok((
        AppendHeaders([(
            header::SET_COOKIE,
            build_auth_cookie(
                token::REFRESH_TOKEN_COOKIE,
                &next_refresh_token,
                REFRESH_TOKEN_MAX_AGE_SECONDS,
                config.github.uses_secure_cookies(),
            ),
        )]),
        Json(TokenResponse {
            access_token: token::issue_access_token(&config.auth, &user)?,
            token_type: "Bearer",
            expires_in: token::ACCESS_TOKEN_TTL_SECONDS,
        }),
    ))
}

/// 吊销浏览器 refresh token 并清除其 Cookie
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "Auth",
    summary = "退出当前会话",
    description = "吊销浏览器中的刷新令牌并清除对应的 HttpOnly Cookie",
    security(("refresh_token" = [])),
    responses(
        (status = 200, body = MessageResponse),
        (status = 400, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(crate) async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let refresh_token = token::read_cookie(&headers, token::REFRESH_TOKEN_COOKIE)
        .ok_or_else(|| AppError::BadRequest("missing refresh token cookie".into()))?;
    state
        .application()
        .revoke_refresh_token(token::hash_sensitive_token(
            &state.config().auth,
            &refresh_token,
        )?)
        .await?;
    Ok((
        AppendHeaders([(
            header::SET_COOKIE,
            build_auth_cookie(
                token::REFRESH_TOKEN_COOKIE,
                "",
                0,
                state.config().github.uses_secure_cookies(),
            ),
        )]),
        Json(MessageResponse {
            message: "ok".into(),
        }),
    ))
}

/// 返回所提供 bearer token 对应的用户资料
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "Auth",
    summary = "获取当前用户",
    description = "校验 Authorization 请求头中的 Bearer 访问令牌并返回当前用户资料",
    security(("access_token" = [])),
    responses(
        (status = 200, body = MeResponse),
        (status = 401, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub(crate) async fn current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, AppError> {
    let bearer_token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("missing bearer token".into()))?;
    let user_id = token::verify_access_token(&state.config().auth, bearer_token)?
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized("invalid access token subject".into()))?;
    let user = state
        .application()
        .find_current_user(user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("invalid user".into()))?;
    Ok(Json(MeResponse {
        id: user.id.to_string(),
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        groups: user.groups,
        is_admin: user.is_admin,
    }))
}

#[cfg(test)]
mod tests {
    use super::build_auth_cookie;

    /// HTTPS 部署生成的认证 Cookie 包含 Secure 属性
    #[test]
    fn adds_secure_attribute_for_https_deployments() {
        let cookie = build_auth_cookie("session", "value", 60, true);

        assert!(cookie.contains("; Secure"));
    }

    /// 本地 HTTP 开发环境仍可接收认证 Cookie
    #[test]
    fn omits_secure_attribute_for_http_development() {
        let cookie = build_auth_cookie("session", "value", 60, false);

        assert!(!cookie.contains("; Secure"));
    }
}
