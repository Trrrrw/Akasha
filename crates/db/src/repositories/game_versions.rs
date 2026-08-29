use std::collections::{HashMap, HashSet};

use akasha_application::game_versions::{
    GameVersion, SyncGameVersionsCommand, SyncGameVersionsResult,
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, DbErr, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, TransactionError,
    TransactionTrait,
    sea_query::{Expr, OnConflict},
};
use serde_json::json;

use crate::{Db, DbError, entities::game_versions, repositories::audit};

/// 按开始时间列出一个游戏的版本时间线
pub async fn list(db: &Db, game_id: &str) -> Result<Vec<GameVersion>, DbError> {
    game_versions::Entity::find()
        .filter(game_versions::Column::GameId.eq(game_id))
        .order_by_asc(game_versions::Column::StartTime)
        .order_by_asc(game_versions::Column::Id)
        .all(db.conn())
        .await
        .map_err(DbError::Query)
        .map(|rows| rows.into_iter().map(GameVersion::from).collect())
}

/// 在同一事务中同步版本时间线和审计日志
pub async fn sync(
    db: &Db,
    command: SyncGameVersionsCommand,
) -> Result<SyncGameVersionsResult, DbError> {
    db.conn()
        .transaction::<_, SyncGameVersionsResult, DbErr>(|txn| {
            Box::pin(async move {
                let now = Utc::now().fixed_offset();
                let existing_versions = game_versions::Entity::find()
                    .filter(game_versions::Column::GameId.eq(&command.game_id))
                    .all(txn)
                    .await?;
                let versions_by_id = existing_versions
                    .iter()
                    .map(|row| (row.id.as_str(), row))
                    .collect::<HashMap<_, _>>();
                let mut version_ids = HashSet::with_capacity(command.versions.len());
                let mut versions_created = 0;
                let mut versions_updated = 0;
                let versions = command
                    .versions
                    .into_iter()
                    .map(|version| {
                        if !version_ids.insert(version.id.clone()) {
                            return Err(DbErr::Custom(format!(
                                "duplicated game version id: {}",
                                version.id
                            )));
                        }
                        let existing = versions_by_id.get(version.id.as_str()).copied();
                        let created_at = existing.map_or(now, |row| row.created_at);
                        let row = game_versions::Model {
                            game_id: command.game_id.clone(),
                            id: version.id,
                            name: version.name,
                            start_time: version.start_time,
                            end_time: existing.and_then(|row| row.end_time),
                            time_status: version.time_status,
                            source_id: version.source_id,
                            source_news_id: version.source_news_id,
                            source_hash: version.source_hash,
                            created_at,
                            updated_at: now,
                        };
                        match existing {
                            None => versions_created += 1,
                            Some(existing) if version_changed(existing, &row) => {
                                versions_updated += 1;
                            }
                            Some(_) => return Ok(None),
                        }
                        Ok(Some(row))
                    })
                    .collect::<Result<Vec<_>, DbErr>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                if !versions.is_empty() {
                    game_versions::Entity::insert_many(
                        versions.into_iter().map(IntoActiveModel::into_active_model),
                    )
                    .on_conflict(version_upsert())
                    .exec(txn)
                    .await?;
                }

                let mut versions_deleted = 0;
                if command.replace {
                    let stale_version_ids = existing_versions
                        .iter()
                        .filter(|row| !version_ids.contains(&row.id))
                        .map(|row| row.id.clone())
                        .collect::<Vec<_>>();
                    versions_deleted = stale_version_ids.len() as u64;
                    if !stale_version_ids.is_empty() {
                        game_versions::Entity::delete_many()
                            .filter(game_versions::Column::GameId.eq(&command.game_id))
                            .filter(game_versions::Column::Id.is_in(stale_version_ids))
                            .exec(txn)
                            .await?;
                    }
                }

                synchronize_ends(txn, &command.game_id, now).await?;
                let changed = versions_created > 0 || versions_updated > 0 || versions_deleted > 0;
                let result = SyncGameVersionsResult {
                    versions_created,
                    versions_updated,
                    versions_deleted,
                    changed,
                };
                audit::insert(
                    txn,
                    &command.audit,
                    "game_versions.sync",
                    Some("game_versions"),
                    Some(command.game_id),
                    json!({
                        "replace": command.replace,
                        "versions_created": versions_created,
                        "versions_updated": versions_updated,
                        "versions_deleted": versions_deleted,
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

async fn synchronize_ends(
    txn: &sea_orm::DatabaseTransaction,
    game_id: &str,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<(), DbErr> {
    let versions = game_versions::Entity::find()
        .filter(game_versions::Column::GameId.eq(game_id))
        .order_by_asc(game_versions::Column::StartTime)
        .order_by_asc(game_versions::Column::Id)
        .all(txn)
        .await?;
    for (index, version) in versions.iter().enumerate() {
        let end_time = versions.get(index + 1).map(|next| next.start_time);
        if version.end_time != end_time {
            game_versions::Entity::update_many()
                .col_expr(game_versions::Column::EndTime, Expr::value(end_time))
                .col_expr(game_versions::Column::UpdatedAt, Expr::value(now))
                .filter(game_versions::Column::GameId.eq(game_id))
                .filter(game_versions::Column::Id.eq(&version.id))
                .exec(txn)
                .await?;
        }
    }
    Ok(())
}

fn version_upsert() -> OnConflict {
    OnConflict::columns([game_versions::Column::GameId, game_versions::Column::Id])
        .update_columns([
            game_versions::Column::Name,
            game_versions::Column::StartTime,
            game_versions::Column::TimeStatus,
            game_versions::Column::SourceId,
            game_versions::Column::SourceNewsId,
            game_versions::Column::SourceHash,
            game_versions::Column::UpdatedAt,
        ])
        .to_owned()
}

fn version_changed(existing: &game_versions::Model, incoming: &game_versions::Model) -> bool {
    existing.name != incoming.name
        || existing.start_time != incoming.start_time
        || existing.time_status != incoming.time_status
        || existing.source_id != incoming.source_id
        || existing.source_news_id != incoming.source_news_id
        || existing.source_hash != incoming.source_hash
}

fn transaction_error(error: TransactionError<DbErr>) -> DbError {
    match error {
        TransactionError::Connection(error) | TransactionError::Transaction(error) => {
            DbError::Query(error)
        }
    }
}

impl From<game_versions::Model> for GameVersion {
    fn from(row: game_versions::Model) -> Self {
        Self {
            game_id: row.game_id,
            id: row.id,
            name: row.name,
            start_time: row.start_time,
            end_time: row.end_time,
            time_status: row.time_status,
            source_id: row.source_id,
            source_news_id: row.source_news_id,
            source_hash: row.source_hash,
        }
    }
}
