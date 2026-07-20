use crate::{
    features::auth::{
        dto::{MeResponse, MessageResponse, TokenResponse},
        github,
        query::GitHubCallbackQuery,
        token,
    },
    http::error::AppError,
    state::AppState,
};
use axum::{
    Json,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, header},
    response::{AppendHeaders, IntoResponse, Redirect},
};
use std::net::SocketAddr;

const COOKIE_PATH: &str = "/api/v1/auth";
fn cookie(name: &str, value: &str, age: u32) -> String {
    format!("{name}={value}; Path={COOKIE_PATH}; Max-Age={age}; HttpOnly; SameSite=Lax")
}
fn meta(headers: &HeaderMap, addr: SocketAddr) -> akasha_db::repositories::auth::RefreshTokenMeta {
    akasha_db::repositories::auth::RefreshTokenMeta {
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(ToOwned::to_owned),
        ip_address: Some(addr.ip().to_string()),
    }
}

pub(crate) async fn github_login(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let value = token::random_token();
    let c = state.config();
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read:user&state={}",
        urlencoding::encode(&c.github.client_id),
        urlencoding::encode(&c.github.redirect_url),
        urlencoding::encode(&value)
    );
    Ok((
        AppendHeaders([(
            header::SET_COOKIE,
            cookie(token::OAUTH_STATE_COOKIE, &value, 600),
        )]),
        Redirect::temporary(&url),
    ))
}
pub(crate) async fn github_callback(
    State(state): State<AppState>,
    Query(query): Query<GitHubCallbackQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    token::validate_state(&headers, &query.state)?;
    let c = state.config();
    let github_token = github::exchange(state.http_client(), &c.github, &query.code).await?;
    let profile = github::user(state.http_client(), &github_token.access_token).await?;
    let user = akasha_db::repositories::auth::upsert_github_user(
        state.db(),
        akasha_db::repositories::auth::GithubUserProfile {
            provider_user_id: profile.id.to_string(),
            provider_login: profile.login.clone(),
            display_name: profile.name.unwrap_or(profile.login),
            email: profile.email,
            avatar_url: profile.avatar_url,
            is_admin: c.github.admin_github_id == Some(profile.id),
        },
    )
    .await
    .map_err(|e| AppError::Internal(e.into()))?;
    let refresh = token::refresh_token();
    let hash = token::hash(&c.auth, &refresh)?;
    akasha_db::repositories::auth::save_refresh_token(
        state.db(),
        user.id,
        hash,
        meta(&headers, addr),
    )
    .await
    .map_err(|e| AppError::Internal(e.into()))?;
    Ok((
        AppendHeaders([
            (header::SET_COOKIE, cookie(token::OAUTH_STATE_COOKIE, "", 0)),
            (
                header::SET_COOKIE,
                cookie(token::REFRESH_TOKEN_COOKIE, &refresh, 2592000),
            ),
        ]),
        Redirect::temporary("/"),
    ))
}
pub(crate) async fn refresh(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let old = token::cookie(&headers, token::REFRESH_TOKEN_COOKIE)
        .ok_or_else(|| AppError::BadRequest("missing refresh token cookie".into()))?;
    let c = state.config();
    let next = token::refresh_token();
    let user = akasha_db::repositories::auth::rotate_refresh_token(
        state.db(),
        token::hash(&c.auth, &old)?,
        token::hash(&c.auth, &next)?,
        meta(&headers, addr),
    )
    .await
    .map_err(|e| AppError::Internal(e.into()))?;
    Ok((
        AppendHeaders([(
            header::SET_COOKIE,
            cookie(token::REFRESH_TOKEN_COOKIE, &next, 2592000),
        )]),
        Json(TokenResponse {
            access_token: token::access_token(&c.auth, &user)?,
            token_type: "Bearer",
            expires_in: 900,
        }),
    ))
}
pub(crate) async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let value = token::cookie(&headers, token::REFRESH_TOKEN_COOKIE)
        .ok_or_else(|| AppError::BadRequest("missing refresh token cookie".into()))?;
    akasha_db::repositories::auth::revoke_refresh_token(
        state.db(),
        token::hash(&state.config().auth, &value)?,
    )
    .await
    .map_err(|e| AppError::Internal(e.into()))?;
    Ok((
        AppendHeaders([(
            header::SET_COOKIE,
            cookie(token::REFRESH_TOKEN_COOKIE, "", 0),
        )]),
        Json(MessageResponse {
            message: "ok".into(),
        }),
    ))
}
pub(crate) async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, AppError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::BadRequest("missing bearer token".into()))?;
    let id = token::verify(&state.config().auth, value)?
        .sub
        .parse()
        .map_err(|_| AppError::BadRequest("invalid user id".into()))?;
    let user = akasha_db::repositories::auth::find_current_user(state.db(), id)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::BadRequest("invalid user".into()))?;
    Ok(Json(MeResponse {
        id: user.id.to_string(),
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        groups: user.groups,
        is_admin: user.is_admin,
    }))
}
