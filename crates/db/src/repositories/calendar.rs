use std::collections::{HashMap, HashSet};

use akasha_application::calendar::{
    CalendarEvent, ListCalendarEventsFilter, SyncCalendarEventsCommand, SyncCalendarEventsResult,
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, DbErr, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect,
    TransactionError, TransactionTrait, sea_query::OnConflict,
};
use serde_json::json;

use crate::{Db, DbError, entities::game_events, repositories::audit};

/// 查询与时间范围相交的游戏活动
pub async fn list_events(
    db: &Db,
    filter: ListCalendarEventsFilter,
) -> Result<Vec<CalendarEvent>, DbError> {
    let mut query = game_events::Entity::find()
        .filter(game_events::Column::GameId.eq(&filter.game_id))
        .filter(game_events::Column::StartTime.lt(filter.end_time))
        .filter(game_events::Column::EndTime.gt(filter.start_time));
    if !filter.kinds.is_empty() {
        query = query.filter(game_events::Column::Kind.is_in(filter.kinds));
    }

    query
        .order_by_asc(game_events::Column::StartTime)
        .order_by_asc(game_events::Column::EndTime)
        .order_by_asc(game_events::Column::Id)
        .limit(filter.limit)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?
        .into_iter()
        .map(event_from_model)
        .collect()
}

/// 在同一事务中同步活动和审计日志
pub async fn sync(
    db: &Db,
    command: SyncCalendarEventsCommand,
) -> Result<SyncCalendarEventsResult, DbError> {
    db.conn()
        .transaction::<_, SyncCalendarEventsResult, DbErr>(|txn| {
            Box::pin(async move {
                let now = Utc::now().fixed_offset();
                let existing_events = game_events::Entity::find()
                    .filter(game_events::Column::GameId.eq(&command.game_id))
                    .all(txn)
                    .await?;
                let events_by_id = existing_events
                    .iter()
                    .map(|row| (row.id.as_str(), row))
                    .collect::<HashMap<_, _>>();

                let mut event_ids = HashSet::with_capacity(command.events.len());
                let mut events_created = 0;
                let mut events_updated = 0;
                let events = command
                    .events
                    .into_iter()
                    .map(|event| {
                        if !event_ids.insert(event.id.clone()) {
                            return Err(DbErr::Custom(format!(
                                "duplicated calendar event id: {}",
                                event.id
                            )));
                        }
                        let existing = events_by_id.get(event.id.as_str()).copied();
                        let created_at = existing.map_or(now, |row| row.created_at);
                        let row = game_events::Model {
                            game_id: command.game_id.clone(),
                            id: event.id,
                            kind: event.kind,
                            title: event.title,
                            start_time: event.start_time,
                            end_time: event.end_time,
                            version_id: event.version_id,
                            start_version_id: event.start_version_id,
                            cover: event.cover,
                            labels: json!(event.labels),
                            source_id: event.source_id,
                            source_news_id: event.source_news_id,
                            source_url: event.source_url,
                            source_hash: event.source_hash,
                            created_at,
                            updated_at: now,
                        };
                        match existing {
                            None => events_created += 1,
                            Some(existing) if event_changed(existing, &row) => events_updated += 1,
                            Some(_) => return Ok(None),
                        }
                        Ok(Some(row))
                    })
                    .collect::<Result<Vec<_>, DbErr>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                if !events.is_empty() {
                    game_events::Entity::insert_many(
                        events.into_iter().map(IntoActiveModel::into_active_model),
                    )
                    .on_conflict(event_upsert())
                    .exec(txn)
                    .await?;
                }

                let mut events_deleted = 0;
                if command.replace {
                    let stale_event_ids = existing_events
                        .iter()
                        .filter(|row| !event_ids.contains(&row.id))
                        .map(|row| row.id.clone())
                        .collect::<Vec<_>>();
                    events_deleted = stale_event_ids.len() as u64;
                    if !stale_event_ids.is_empty() {
                        game_events::Entity::delete_many()
                            .filter(game_events::Column::GameId.eq(&command.game_id))
                            .filter(game_events::Column::Id.is_in(stale_event_ids))
                            .exec(txn)
                            .await?;
                    }
                }

                let changed = events_created > 0 || events_updated > 0 || events_deleted > 0;
                let result = SyncCalendarEventsResult {
                    events_created,
                    events_updated,
                    events_deleted,
                    changed,
                };
                audit::insert(
                    txn,
                    &command.audit,
                    "calendar_events.sync",
                    Some("calendar"),
                    Some(command.game_id),
                    json!({
                        "replace": command.replace,
                        "events_created": events_created,
                        "events_updated": events_updated,
                        "events_deleted": events_deleted,
                        "changed": changed,
                    }),
                )
                .await?;
                Ok(result)
            })
        })
        .await
        .map_err(transaction_error)
}

fn event_upsert() -> OnConflict {
    OnConflict::columns([game_events::Column::GameId, game_events::Column::Id])
        .update_columns([
            game_events::Column::Kind,
            game_events::Column::Title,
            game_events::Column::StartTime,
            game_events::Column::EndTime,
            game_events::Column::VersionId,
            game_events::Column::StartVersionId,
            game_events::Column::Cover,
            game_events::Column::Labels,
            game_events::Column::SourceId,
            game_events::Column::SourceNewsId,
            game_events::Column::SourceUrl,
            game_events::Column::SourceHash,
            game_events::Column::UpdatedAt,
        ])
        .to_owned()
}

fn event_changed(existing: &game_events::Model, incoming: &game_events::Model) -> bool {
    existing.kind != incoming.kind
        || existing.title != incoming.title
        || existing.start_time != incoming.start_time
        || existing.end_time != incoming.end_time
        || existing.version_id != incoming.version_id
        || existing.start_version_id != incoming.start_version_id
        || existing.cover != incoming.cover
        || existing.labels != incoming.labels
        || existing.source_id != incoming.source_id
        || existing.source_news_id != incoming.source_news_id
        || existing.source_url != incoming.source_url
        || existing.source_hash != incoming.source_hash
}

fn event_from_model(row: game_events::Model) -> Result<CalendarEvent, DbError> {
    let labels = serde_json::from_value(row.labels).map_err(|error| {
        DbError::Query(DbErr::Custom(format!(
            "invalid calendar event labels: {error}"
        )))
    })?;
    Ok(CalendarEvent {
        game_id: row.game_id,
        id: row.id,
        kind: row.kind,
        title: row.title,
        start_time: row.start_time,
        end_time: row.end_time,
        version_id: row.version_id,
        start_version_id: row.start_version_id,
        cover: row.cover,
        labels,
        source_id: row.source_id,
        source_news_id: row.source_news_id,
        source_url: row.source_url,
        source_hash: row.source_hash,
    })
}

fn transaction_error(error: TransactionError<DbErr>) -> DbError {
    match error {
        TransactionError::Connection(error) | TransactionError::Transaction(error) => {
            DbError::Query(error)
        }
    }
}
