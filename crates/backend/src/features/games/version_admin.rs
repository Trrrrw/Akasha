use akasha_application::game_versions::{
    GameVersionInput, SyncGameVersionsCommand, SyncGameVersionsResult,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::{
    http::{
        error::AppError,
        extractors::{AuditRequest, DataWriteActor},
        path::require_game,
        response::utc_timestamp,
    },
    state::AppState,
};

const MAX_VERSIONS_PER_SYNC: usize = 256;

#[derive(Deserialize)]
pub(crate) struct SyncGameVersionsRequest {
    replace: bool,
    versions: Vec<GameVersionRequest>,
    audit: Option<AuditRequest>,
}

#[derive(Deserialize)]
struct GameVersionRequest {
    id: String,
    name: Option<String>,
    start_time: DateTime<FixedOffset>,
    time_status: String,
    source_id: String,
    source_news_id: String,
    source_hash: String,
}

#[derive(Serialize)]
pub(crate) struct GameVersionRawResponse {
    id: String,
    name: Option<String>,
    start_time: String,
    end_time: Option<String>,
    time_status: String,
    source_id: String,
    source_news_id: String,
    source_hash: String,
}

#[derive(Serialize)]
pub(crate) struct SyncGameVersionsResponse {
    versions_created: u64,
    versions_updated: u64,
    versions_deleted: u64,
    changed: bool,
}

/// 读取 Worker 使用的原始游戏版本时间线
pub(crate) async fn list_raw(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<Json<Vec<GameVersionRawResponse>>, AppError> {
    require_game(&state, &game_id).await?;
    tracing::debug!(actor = %actor.label(), game_id, "listing raw game versions for worker");
    let items = state
        .application()
        .list_game_versions(&game_id)
        .await?
        .into_iter()
        .map(|version| GameVersionRawResponse {
            id: version.id,
            name: version.name,
            start_time: utc_timestamp(version.start_time),
            end_time: version.end_time.map(utc_timestamp),
            time_status: version.time_status,
            source_id: version.source_id,
            source_news_id: version.source_news_id,
            source_hash: version.source_hash,
        })
        .collect();
    Ok(Json(items))
}

/// 同步一个游戏的版本时间线
pub(crate) async fn sync(
    actor: DataWriteActor,
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(game_id): Path<String>,
    Json(body): Json<SyncGameVersionsRequest>,
) -> Result<Json<SyncGameVersionsResponse>, AppError> {
    if body.versions.len() > MAX_VERSIONS_PER_SYNC {
        return Err(AppError::BadRequest(
            "game version sync payload contains too many items".to_owned(),
        ));
    }
    require_game(&state, &game_id).await?;
    let audit = actor.audit_context(body.audit.unwrap_or_default(), &headers);
    let result = state
        .application()
        .sync_game_versions(SyncGameVersionsCommand {
            game_id,
            replace: body.replace,
            versions: body
                .versions
                .into_iter()
                .map(GameVersionInput::from)
                .collect(),
            audit,
        })
        .await?;
    Ok(Json(result.into()))
}

impl From<GameVersionRequest> for GameVersionInput {
    fn from(value: GameVersionRequest) -> Self {
        Self {
            id: value.id,
            name: value.name,
            start_time: value.start_time,
            time_status: value.time_status,
            source_id: value.source_id,
            source_news_id: value.source_news_id,
            source_hash: value.source_hash,
        }
    }
}

impl From<SyncGameVersionsResult> for SyncGameVersionsResponse {
    fn from(value: SyncGameVersionsResult) -> Self {
        Self {
            versions_created: value.versions_created,
            versions_updated: value.versions_updated,
            versions_deleted: value.versions_deleted,
            changed: value.changed,
        }
    }
}
