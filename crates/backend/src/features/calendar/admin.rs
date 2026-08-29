use akasha_application::calendar::{
    CalendarEventInput, SyncCalendarEventsCommand, SyncCalendarEventsResult,
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
    },
    state::AppState,
};

const MAX_EVENTS_PER_SYNC: usize = 10_000;
const MAX_LABELS_PER_EVENT: usize = 16;

#[derive(Deserialize)]
pub(crate) struct SyncCalendarEventsRequest {
    replace: bool,
    events: Vec<CalendarEventRequest>,
    audit: Option<AuditRequest>,
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
pub(crate) struct SyncCalendarEventsResponse {
    events_created: u64,
    events_updated: u64,
    events_deleted: u64,
    changed: bool,
}

/// 同步一个游戏的活动投影
pub(crate) async fn sync_events(
    actor: DataWriteActor,
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(game_id): Path<String>,
    Json(body): Json<SyncCalendarEventsRequest>,
) -> Result<Json<SyncCalendarEventsResponse>, AppError> {
    validate_request(&body)?;
    require_game(&state, &game_id).await?;
    let audit = actor.audit_context(body.audit.unwrap_or_default(), &headers);
    let result = state
        .application()
        .sync_calendar_events(SyncCalendarEventsCommand {
            game_id,
            replace: body.replace,
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

fn validate_request(body: &SyncCalendarEventsRequest) -> Result<(), AppError> {
    if body.events.len() > MAX_EVENTS_PER_SYNC {
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

impl From<SyncCalendarEventsResult> for SyncCalendarEventsResponse {
    fn from(value: SyncCalendarEventsResult) -> Self {
        Self {
            events_created: value.events_created,
            events_updated: value.events_updated,
            events_deleted: value.events_deleted,
            changed: value.changed,
        }
    }
}
