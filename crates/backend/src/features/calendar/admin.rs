use akasha_application::calendar::{
    CalendarEventInput, SyncCalendarEventsCommand, SyncCalendarEventsResult,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
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
    entries: Vec<CalendarEventInput>,
    audit: Option<AuditRequest>,
}

#[derive(Serialize)]
pub(crate) struct SyncCalendarEventsResponse {
    entries_created: u64,
    entries_updated: u64,
    entries_deleted: u64,
    changed: bool,
}

/// 同步一个游戏的日程投影
pub(crate) async fn sync_entries(
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
            events: body.entries,
            audit,
        })
        .await?;
    Ok(Json(result.into()))
}

fn validate_request(body: &SyncCalendarEventsRequest) -> Result<(), AppError> {
    if body.entries.len() > MAX_EVENTS_PER_SYNC {
        return Err(AppError::BadRequest(
            "calendar sync payload contains too many items".to_owned(),
        ));
    }
    if body.entries.iter().any(|event| {
        event.labels.len() > MAX_LABELS_PER_EVENT
            || event.title.chars().count() > 256
            || event.labels.iter().any(|label| label.chars().count() > 64)
    }) {
        return Err(AppError::BadRequest(
            "calendar entry fields exceed their limits".to_owned(),
        ));
    }
    Ok(())
}

impl From<SyncCalendarEventsResult> for SyncCalendarEventsResponse {
    fn from(value: SyncCalendarEventsResult) -> Self {
        Self {
            entries_created: value.events_created,
            entries_updated: value.events_updated,
            entries_deleted: value.events_deleted,
            changed: value.changed,
        }
    }
}
