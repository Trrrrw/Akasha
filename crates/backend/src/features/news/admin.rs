use akasha_application::news::{
    ListNewsRawFilter, NewsCharacterInput, NewsCharacterUpdate, NewsTagInput, NewsTagUpdate,
    ReplaceNewsCharactersCommand, ReplaceNewsTagsCommand, SyncNewsTagsCommand, UpdateNewsCommand,
    VideoPlayback,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::HeaderName},
    response::IntoResponse,
};
use axum_extra::extract::Query as MultiQuery;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::dto::NewsItemResponse;
use crate::{
    http::{
        error::AppError,
        extractors::{AuditRequest, DataWriteActor},
        path::{GamePath, require_news_source},
        response::utc_timestamp,
    },
    state::AppState,
};

const AKASHA_CHANGE_HEADER: HeaderName = HeaderName::from_static("x-akasha-change");

/// 创建或更新新闻的 HTTP 请求体
#[derive(Deserialize)]
pub(crate) struct UpdateNewsRequest {
    source_id: String,
    id: String,
    title: String,
    intro: Option<String>,
    publish_time: DateTime<FixedOffset>,
    source_url: String,
    cover: Option<String>,
    news_type: String,
    video_url: Option<String>,
    video_playback: Option<String>,
    /// 视频时长，单位为毫秒
    video_duration_ms: Option<i64>,
    tags: Vec<String>,
    /// 角色关联，缺省时保留已有关系
    characters: Option<Vec<UpdateNewsCharacterRequest>>,
    raw_data: Value,
    audit: Option<AuditRequest>,
}

/// 读取维护任务所需原始新闻的管理查询参数
#[derive(Deserialize)]
pub(crate) struct NewsRawQuery {
    source_id: String,
    id: Option<String>,
    after_id: Option<String>,
    news_type: Option<String>,
    #[serde(default)]
    tag: Vec<String>,
    limit: Option<u64>,
}

/// 原始新闻管理接口的分页响应
#[derive(Serialize)]
pub(crate) struct NewsRawPageResponse {
    total: u64,
    limit: u64,
    items: Vec<NewsRawItemResponse>,
    next_cursor: Option<String>,
}

/// 原始新闻管理接口中的单条记录
#[derive(Serialize)]
pub(crate) struct NewsRawItemResponse {
    id: String,
    title: String,
    intro: Option<String>,
    publish_time: String,
    source_url: String,
    cover: Option<String>,
    news_type: String,
    tags: Vec<String>,
    video_url: Option<String>,
    video_playback: Option<String>,
    video_duration_ms: Option<i64>,
    raw_data: Value,
}

/// 新闻写入请求中的角色关联
#[derive(Deserialize)]
pub(crate) struct UpdateNewsCharacterRequest {
    id: String,
    name: String,
}

/// 读取 worker 重新解析所需的原始新闻
pub(crate) async fn list_raw(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
    MultiQuery(query): MultiQuery<NewsRawQuery>,
) -> Result<Json<NewsRawPageResponse>, AppError> {
    tracing::debug!(
        actor = %actor.label(),
        game_id = %game_id,
        source_id = %query.source_id,
        "listing raw news for maintenance"
    );

    if query.id.is_some() && query.after_id.is_some() {
        return Err(AppError::BadRequest(
            "id and after_id cannot be used together".into(),
        ));
    }
    let source_id = normalized_source_id(&query.source_id)?;
    let news_type = normalized_optional_news_type(query.news_type)?;
    require_news_source(&state, &game_id, &source_id).await?;

    let limit = query.limit.unwrap_or(100).clamp(1, 100);
    let (total, items) = state
        .application()
        .list_news_raw(ListNewsRawFilter {
            game_id,
            source_id,
            news_id: query.id,
            after_id: query.after_id,
            news_type,
            tags: query
                .tag
                .into_iter()
                .map(|tag| tag.trim().to_owned())
                .filter(|tag| !tag.is_empty())
                .collect(),
            limit,
        })
        .await?;
    let next_cursor = items.last().map(|item| item.id.clone());

    Ok(Json(NewsRawPageResponse {
        total,
        limit,
        items: items.into_iter().map(NewsRawItemResponse::from).collect(),
        next_cursor,
    }))
}

impl From<akasha_application::news::NewsRawItem> for NewsRawItemResponse {
    /// 将应用层原始新闻转换为维护接口响应
    fn from(value: akasha_application::news::NewsRawItem) -> Self {
        Self {
            id: value.id,
            title: value.title,
            intro: value.intro,
            publish_time: utc_timestamp(value.publish_time),
            source_url: value.source_url,
            cover: value.cover,
            news_type: value.news_type,
            tags: value.tags,
            video_url: value.video_url,
            video_playback: value
                .video_playback
                .map(|playback| playback.as_str().to_owned()),
            video_duration_ms: value.video_duration_ms,
            raw_data: value.raw_data,
        }
    }
}

/// 从受信任数据来源创建或更新一条新闻
pub(crate) async fn update_news(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
    headers: HeaderMap,
    Json(body): Json<UpdateNewsRequest>,
) -> Result<impl IntoResponse, AppError> {
    let source_id = normalized_source_id(&body.source_id)?;
    let news_type = normalized_news_type(&body.news_type)?;
    let video_playback = normalized_video_playback(
        body.video_playback.as_deref(),
        &news_type,
        body.video_url.as_deref(),
    )?;
    require_news_source(&state, &game_id, &source_id).await?;
    let audit = actor.audit_context(body.audit.unwrap_or_default(), &headers);
    let news_id = body.id.clone();
    let result = state
        .application()
        .update_news(UpdateNewsCommand {
            game_id: game_id.clone(),
            source_id,
            id: body.id,
            title: body.title,
            intro: body.intro,
            publish_time: body.publish_time,
            source_url: body.source_url,
            cover: body.cover,
            news_type,
            video_url: body.video_url,
            video_playback,
            video_duration_ms: body.video_duration_ms,
            tags: body.tags,
            characters: body.characters.map(|characters| {
                characters
                    .into_iter()
                    .map(|character| NewsCharacterInput {
                        id: character.id,
                        name: character.name,
                    })
                    .collect()
            }),
            raw_data: body.raw_data,
            audit,
        })
        .await?;

    if result.changed {
        tracing::info!(
            actor = %actor.label(),
            game_id = %game_id,
            news_id = %news_id,
            created = result.created,
            "saved news"
        );
    } else {
        tracing::debug!(
            actor = %actor.label(),
            game_id = %game_id,
            news_id = %news_id,
            "news is unchanged"
        );
    }

    let status = if result.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    let change = if result.created {
        HeaderValue::from_static("created")
    } else if result.changed {
        HeaderValue::from_static("updated")
    } else {
        HeaderValue::from_static("unchanged")
    };
    Ok((
        status,
        [(AKASHA_CHANGE_HEADER, change)],
        Json(NewsItemResponse::from_summary(
            result.item,
            result.game_cover.as_deref(),
            &state.config().asset_base_url,
        )),
    ))
}

/// 同步新闻来源标签目录的 HTTP 请求体
#[derive(Deserialize)]
pub(crate) struct SyncTagsRequest {
    source_id: String,
    tags: Vec<SyncNewsTagRequest>,
    audit: Option<AuditRequest>,
}

/// 标签目录同步请求中的单个标签
#[derive(Deserialize)]
pub(crate) struct SyncNewsTagRequest {
    name: String,
    index: i64,
    group: Option<String>,
    group_index: Option<i64>,
}

/// 标签目录同步后的 HTTP 响应
#[derive(Serialize)]
pub(crate) struct SyncTagsResponse {
    changed: bool,
    tags: Vec<SyncNewsTagResponse>,
}

/// 标签目录同步响应中的单个标签
#[derive(Serialize)]
pub(crate) struct SyncNewsTagResponse {
    name: String,
    index: i64,
    group: Option<String>,
    group_index: Option<i64>,
}

/// 同步一个游戏新闻来源的标签目录
pub(crate) async fn sync_tags(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
    headers: HeaderMap,
    Json(body): Json<SyncTagsRequest>,
) -> Result<Json<SyncTagsResponse>, AppError> {
    let source_id = normalized_source_id(&body.source_id)?;
    require_news_source(&state, &game_id, &source_id).await?;
    let audit = actor.audit_context(body.audit.unwrap_or_default(), &headers);
    tracing::info!(
        actor = %actor.label(),
        game_id = %game_id,
        source_id = %source_id,
        tags = body.tags.len(),
        "syncing news tags"
    );

    let result = state
        .application()
        .sync_news_tags(SyncNewsTagsCommand {
            game_id,
            source_id,
            tags: body
                .tags
                .into_iter()
                .map(|tag| NewsTagInput {
                    name: tag.name,
                    index: tag.index,
                    group: tag.group,
                    group_index: tag.group_index,
                })
                .collect(),
            audit,
        })
        .await?;

    Ok(Json(SyncTagsResponse {
        changed: result.changed,
        tags: result
            .tags
            .into_iter()
            .map(|tag| SyncNewsTagResponse {
                name: tag.name,
                index: tag.index,
                group: tag.group,
                group_index: tag.group_index,
            })
            .collect(),
    }))
}

/// 替换同一来源多条新闻的标签关联
pub(crate) async fn update_tags(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
    headers: HeaderMap,
    Json(body): Json<UpdateNewsTagsRequest>,
) -> Result<StatusCode, AppError> {
    let source_id = normalized_source_id(&body.source_id)?;
    require_news_source(&state, &game_id, &source_id).await?;
    let audit = actor.audit_context(body.audit.unwrap_or_default(), &headers);
    tracing::info!(
        actor = %actor.label(),
        game_id = %game_id,
        source_id = %source_id,
        updates = body.updates.len(),
        "updating news tags"
    );

    state
        .application()
        .replace_news_tags(ReplaceNewsTagsCommand {
            game_id,
            source_id,
            updates: body
                .updates
                .into_iter()
                .map(|update| NewsTagUpdate {
                    id: update.id,
                    tags: update.tags,
                })
                .collect(),
            audit,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// 替换多个新闻标签集合的 HTTP 请求体
#[derive(Deserialize)]
pub(crate) struct UpdateNewsTagsRequest {
    source_id: String,
    updates: Vec<UpdateNewsTagsItemRequest>,
    audit: Option<AuditRequest>,
}

/// HTTP 请求体中一条新闻的替换标签集合
#[derive(Deserialize)]
pub(crate) struct UpdateNewsTagsItemRequest {
    id: String,
    tags: Vec<String>,
}

/// 替换多个新闻角色集合的 HTTP 请求体
#[derive(Deserialize)]
pub(crate) struct UpdateNewsCharactersRequest {
    source_id: String,
    updates: Vec<UpdateNewsCharactersItemRequest>,
    audit: Option<AuditRequest>,
}

/// HTTP 请求体中一条新闻的替换角色集合
#[derive(Deserialize)]
pub(crate) struct UpdateNewsCharactersItemRequest {
    id: String,
    characters: Vec<UpdateNewsCharacterRequest>,
}

/// 替换同一来源多条新闻的角色关联
pub(crate) async fn update_characters(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Path(GamePath { game_id }): Path<GamePath>,
    headers: HeaderMap,
    Json(body): Json<UpdateNewsCharactersRequest>,
) -> Result<StatusCode, AppError> {
    let source_id = normalized_source_id(&body.source_id)?;
    require_news_source(&state, &game_id, &source_id).await?;
    let audit = actor.audit_context(body.audit.unwrap_or_default(), &headers);
    tracing::info!(
        actor = %actor.label(),
        game_id = %game_id,
        source_id = %source_id,
        updates = body.updates.len(),
        "updating news characters"
    );

    state
        .application()
        .replace_news_characters(ReplaceNewsCharactersCommand {
            game_id,
            source_id,
            updates: body
                .updates
                .into_iter()
                .map(|update| NewsCharacterUpdate {
                    id: update.id,
                    characters: update
                        .characters
                        .into_iter()
                        .map(|character| NewsCharacterInput {
                            id: character.id,
                            name: character.name,
                        })
                        .collect(),
                })
                .collect(),
            audit,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

fn normalized_source_id(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(
            "source_id must not be empty".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

fn normalized_news_type(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if matches!(value, "article" | "video") {
        Ok(value.to_owned())
    } else {
        Err(AppError::BadRequest(
            "news_type must be article or video".to_owned(),
        ))
    }
}

fn normalized_optional_news_type(value: Option<String>) -> Result<Option<String>, AppError> {
    value.as_deref().map(normalized_news_type).transpose()
}

/// 校验并兼容旧写入客户端缺省的视频播放方式
fn normalized_video_playback(
    value: Option<&str>,
    news_type: &str,
    video_url: Option<&str>,
) -> Result<Option<VideoPlayback>, AppError> {
    let has_video_url = video_url.is_some_and(|url| !url.trim().is_empty());
    let playback = match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some("direct") => Some(VideoPlayback::Direct),
        Some("embed") => Some(VideoPlayback::Embed),
        Some(_) => {
            return Err(AppError::BadRequest(
                "video_playback must be direct or embed".to_owned(),
            ));
        }
        None if news_type == "video" && has_video_url => Some(VideoPlayback::Direct),
        None => None,
    };

    if playback.is_some() && news_type != "video" {
        return Err(AppError::BadRequest(
            "video_playback is only valid for video news".to_owned(),
        ));
    }
    if playback.is_some() && !has_video_url {
        return Err(AppError::BadRequest(
            "video_playback requires video_url".to_owned(),
        ));
    }

    Ok(playback)
}

#[cfg(test)]
mod tests {
    use akasha_application::news::VideoPlayback;

    use super::normalized_video_playback;
    use crate::http::error::AppError;

    /// 旧视频写入缺省播放方式时保持直接播放兼容语义
    #[test]
    fn defaults_existing_video_contract_to_direct_playback() {
        assert_eq!(
            normalized_video_playback(None, "video", Some("https://example.com/video.mp4"))
                .expect("legacy video request should remain valid"),
            Some(VideoPlayback::Direct)
        );
    }

    /// 嵌入播放必须同时提供视频地址
    #[test]
    fn rejects_embed_playback_without_video_url() {
        let error = normalized_video_playback(Some("embed"), "video", None)
            .expect_err("embed playback without URL should be rejected");

        assert!(matches!(
            error,
            AppError::BadRequest(message) if message == "video_playback requires video_url"
        ));
    }
}
