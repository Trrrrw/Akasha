use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::dto::NewsItemResponse;
use crate::{
    http::{error::AppError, extractors::DataWriteActor},
    state::AppState,
};

#[derive(Deserialize)]
pub(crate) struct UpdateNewsRequest {
    game_id: String,
    source_id: String,
    id: String,
    title: String,
    intro: Option<String>,
    publish_time: DateTime<FixedOffset>,
    source_url: String,
    cover: Option<String>,
    news_type: String,
    video_url: Option<String>,
    tags: Vec<String>,
    raw_data: Value,
}

pub(crate) async fn update_news(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<UpdateNewsRequest>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(actor = %actor.label(), news_id = %body.id, "updating news");
    let game_id = body.game_id.clone();

    let result = akasha_db::repositories::news::update_news(
        state.db(),
        akasha_db::repositories::news::UpdateNewsInput {
            game_id: body.game_id,
            source_id: body.source_id,
            id: body.id,
            title: body.title,
            intro: body.intro,
            publish_time: body.publish_time,
            source_url: body.source_url,
            cover: body.cover,
            news_type: body.news_type,
            video_url: body.video_url,
            tags: body.tags,
            raw_data: body.raw_data,
        },
    )
    .await
    .map_err(|err| AppError::Internal(err.into()))?;

    let status = if result.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    let game_cover = akasha_db::repositories::games::find_cover_by_id(state.db(), &game_id)
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    Ok((
        status,
        Json(NewsItemResponse::from_summary(
            result.news,
            game_cover.as_deref(),
        )),
    ))
}

#[derive(Deserialize)]
pub(crate) struct SyncTagsRequest {
    game_id: String,
    source_id: String,
    tags: Vec<SyncNewsTagRequest>,
}

#[derive(Deserialize)]
pub(crate) struct SyncNewsTagRequest {
    name: String,
    index: i64,
    group: Option<String>,
    group_index: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct SyncTagsResponse {
    changed: bool,
    tags: Vec<SyncNewsTagResponse>,
}

#[derive(Serialize)]
pub(crate) struct SyncNewsTagResponse {
    name: String,
    index: i64,
    group: Option<String>,
    group_index: Option<i64>,
}

pub(crate) async fn sync_tags(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<SyncTagsRequest>,
) -> Result<Json<SyncTagsResponse>, AppError> {
    tracing::info!(
        actor = %actor.label(),
        game_id = %body.game_id,
        source_id = %body.source_id,
        tags = body.tags.len(),
        "syncing news tags"
    );

    let result = akasha_db::repositories::news_tags::sync_tags(
        state.db(),
        akasha_db::repositories::news_tags::SyncTagsInput {
            game_id: body.game_id,
            source_id: body.source_id,
            tags: body
                .tags
                .into_iter()
                .map(|tag| akasha_db::repositories::news_tags::NewsTagInput {
                    name: tag.name,
                    index: tag.index,
                    group: tag.group,
                    group_index: tag.group_index,
                })
                .collect(),
        },
    )
    .await
    .map_err(|err| AppError::Internal(err.into()))?;

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

/// 批量替换一个新闻来源下已有新闻的标签
pub(crate) async fn update_tags(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<UpdateNewsTagsRequest>,
) -> Result<StatusCode, AppError> {
    tracing::info!(
        actor = %actor.label(),
        game_id = %body.game_id,
        source_id = %body.source_id,
        updates = body.updates.len(),
        "updating news tags"
    );

    akasha_db::repositories::news::update_tags(
        state.db(),
        akasha_db::repositories::news::UpdateNewsTagsInput {
            game_id: body.game_id,
            source_id: body.source_id,
            updates: body
                .updates
                .into_iter()
                .map(|update| akasha_db::repositories::news::UpdateNewsTagsItem {
                    id: update.id,
                    tags: update.tags,
                })
                .collect(),
        },
    )
    .await
    .map_err(|err| AppError::Internal(err.into()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// 批量标签替换请求
#[derive(Deserialize)]
pub(crate) struct UpdateNewsTagsRequest {
    game_id: String,
    source_id: String,
    updates: Vec<UpdateNewsTagsItemRequest>,
}

/// 一条新闻的标签替换内容
#[derive(Deserialize)]
pub(crate) struct UpdateNewsTagsItemRequest {
    id: String,
    tags: Vec<String>,
}
