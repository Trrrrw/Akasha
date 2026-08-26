use akasha_application::calendar::{
    CalendarEventInput, GameVersionInput, SyncCalendarCommand, SyncCalendarResult,
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
const MAX_EVENTS_PER_SYNC: usize = 10_000;
const MAX_LABELS_PER_EVENT: usize = 16;

#[derive(Deserialize)]
pub(crate) struct SyncCalendarRequest {
    replace: bool,
    versions: Vec<GameVersionRequest>,
    events: Vec<CalendarEventRequest>,
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

#[derive(Deserialize)]
struct CalendarEventRequest {
    id: String,
    kind: String,
    title: String,
    start_time: DateTime<FixedOffset>,
    end_time: DateTime<FixedOffset>,
    version_id: Option<String>,
    start_version_id: Option<String>,
    cover: Option<String>,
    labels: Vec<String>,
    source_id: String,
    source_news_id: String,
    source_url: String,
    source_hash: String,
}

#[derive(Serialize)]
pub(crate) struct GameVersionResponse {
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
pub(crate) struct SyncCalendarResponse {
    versions_created: u64,
    versions_updated: u64,
    versions_deleted: u64,
    events_created: u64,
    events_updated: u64,
    events_deleted: u64,
    changed: bool,
}

/// 读取 Worker 解析相对时间所需的版本目录
pub(crate) async fn list_versions(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<Json<Vec<GameVersionResponse>>, AppError> {
    require_game(&state, &game_id).await?;
    tracing::debug!(actor = %actor.label(), game_id, "listing game versions for worker");
    let items = state
        .application()
        .list_game_versions(&game_id)
        .await?
        .into_iter()
        .map(|version| GameVersionResponse {
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

/// 原子同步一个游戏的版本和活动投影
pub(crate) async fn sync_calendar(
    actor: DataWriteActor,
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(game_id): Path<String>,
    Json(body): Json<SyncCalendarRequest>,
) -> Result<Json<SyncCalendarResponse>, AppError> {
    validate_request(&body)?;
    require_game(&state, &game_id).await?;
    let audit = actor.audit_context(body.audit.unwrap_or_default(), &headers);
    let result = state
        .application()
        .sync_calendar(SyncCalendarCommand {
            game_id,
            replace: body.replace,
            versions: body
                .versions
                .into_iter()
                .map(GameVersionInput::from)
                .collect(),
            events: body
                .events
                .into_iter()
                .map(CalendarEventInput::from)
                .collect(),
            audit,
        })
        .await?;
    Ok(Json(result.into()))
}

fn validate_request(body: &SyncCalendarRequest) -> Result<(), AppError> {
    if body.versions.len() > MAX_VERSIONS_PER_SYNC || body.events.len() > MAX_EVENTS_PER_SYNC {
        return Err(AppError::BadRequest(
            "calendar sync payload contains too many items".to_owned(),
        ));
    }
    if body.events.iter().any(|event| {
        event.labels.len() > MAX_LABELS_PER_EVENT
            || event.title.chars().count() > 256
            || event.labels.iter().any(|label| label.chars().count() > 64)
    }) {
        return Err(AppError::BadRequest(
            "calendar event fields exceed their limits".to_owned(),
        ));
    }
    Ok(())
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

impl From<CalendarEventRequest> for CalendarEventInput {
    fn from(value: CalendarEventRequest) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            title: value.title,
            start_time: value.start_time,
            end_time: value.end_time,
            version_id: value.version_id,
            start_version_id: value.start_version_id,
            cover: value.cover,
            labels: value.labels,
            source_id: value.source_id,
            source_news_id: value.source_news_id,
            source_url: value.source_url,
            source_hash: value.source_hash,
        }
    }
}

impl From<SyncCalendarResult> for SyncCalendarResponse {
    fn from(value: SyncCalendarResult) -> Self {
        Self {
            versions_created: value.versions_created,
            versions_updated: value.versions_updated,
            versions_deleted: value.versions_deleted,
            events_created: value.events_created,
            events_updated: value.events_updated,
            events_deleted: value.events_deleted,
            changed: value.changed,
        }
    }
}
