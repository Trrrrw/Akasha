use akasha_application::game_data::{
    GameDataCollection, GameDataCollectionFilter, GameDataEntry, GameDataListFilter,
    GameDataRawItem, ListGameDataRawFilter, SyncGameDataCollectionCommand,
    SyncGameDataCollectionResult, UpdateGameDataCollectionCommand,
};

use crate::{Db, DbError};

macro_rules! game_data_repository {
    ($module:ident, $entity:ident, $character_link:ident, $game_id:literal) => {
        mod $module {
            use std::collections::{HashMap, HashSet};

            use sea_orm::{
                ColumnTrait, DbErr, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter,
                QueryOrder, QuerySelect, TransactionError, TransactionTrait,
                sea_query::{Expr, ExprTrait, OnConflict},
            };
            use serde_json::json;

            use super::*;
            use crate::{
                entities::{$character_link, $entity},
                models::text_query_condition,
                repositories::audit,
            };

            pub async fn list_collections(
                db: &Db,
            ) -> Result<Vec<GameDataCollection>, DbError> {
                let rows = $entity::Entity::find()
                    .select_only()
                    .column($entity::Column::Collection)
                    .column_as(Expr::col($entity::Column::Id).count(), "total")
                    .group_by($entity::Column::Collection)
                    .order_by_asc($entity::Column::Collection)
                    .into_tuple::<(String, i64)>()
                    .all(db.conn())
                    .await
                    .map_err(DbError::Query)?;

                Ok(rows
                    .into_iter()
                    .map(|(id, total)| GameDataCollection {
                        id,
                        total: total as u64,
                    })
                    .collect())
            }

            pub async fn list(
                db: &Db,
                filter: GameDataListFilter,
            ) -> Result<(u64, Vec<GameDataEntry>), DbError> {
                let mut query = $entity::Entity::find()
                    .filter($entity::Column::Collection.eq(&filter.collection));
                if let Some(text_query) = filter.query.as_ref() {
                    query = query.filter(text_query_condition(
                        text_query,
                        &[Expr::col($entity::Column::Name)],
                    ));
                }
                let total = query
                    .clone()
                    .count(db.conn())
                    .await
                    .map_err(DbError::Query)?;
                let rows = query
                    .order_by_asc($entity::Column::Name)
                    .order_by_asc($entity::Column::Id)
                    .limit(filter.limit)
                    .offset(filter.offset)
                    .all(db.conn())
                    .await
                    .map_err(DbError::Query)?;
                Ok((total, rows.into_iter().map(GameDataEntry::from).collect()))
            }

            pub async fn find(
                db: &Db,
                collection: &str,
                id: &str,
            ) -> Result<Option<GameDataEntry>, DbError> {
                Ok($entity::Entity::find_by_id((collection.to_owned(), id.to_owned()))
                    .one(db.conn())
                    .await
                    .map_err(DbError::Query)?
                    .map(GameDataEntry::from))
            }

            pub async fn list_raw(
                db: &Db,
                filter: ListGameDataRawFilter,
            ) -> Result<(u64, Vec<GameDataRawItem>), DbError> {
                let base = $entity::Entity::find()
                    .filter($entity::Column::Collection.eq(&filter.collection));
                let total = base
                    .clone()
                    .count(db.conn())
                    .await
                    .map_err(DbError::Query)?;
                let mut query = base;
                if let Some(after_id) = filter.after_id {
                    query = query.filter($entity::Column::Id.gt(after_id));
                }
                let query = query
                    .order_by_asc($entity::Column::Id)
                    .limit(filter.limit);
                let items = if filter.include_raw_data {
                    query
                        .all(db.conn())
                        .await
                        .map_err(DbError::Query)?
                        .into_iter()
                        .map(|row| GameDataRawItem {
                            id: row.id,
                            raw_data: row.raw_data,
                            source_hash: row.source_hash,
                        })
                        .collect()
                } else {
                    query
                        .select_only()
                        .column($entity::Column::Id)
                        .column($entity::Column::SourceHash)
                        .into_tuple::<(String, Option<String>)>()
                        .all(db.conn())
                        .await
                        .map_err(DbError::Query)?
                        .into_iter()
                        .map(|(id, source_hash)| GameDataRawItem {
                            id,
                            raw_data: None,
                            source_hash,
                        })
                        .collect()
                };
                Ok((total, items))
            }

            pub async fn sync(
                db: &Db,
                command: SyncGameDataCollectionCommand,
            ) -> Result<SyncGameDataCollectionResult, DbError> {
                db.conn()
                    .transaction::<_, SyncGameDataCollectionResult, DbErr>(|txn| {
                        Box::pin(async move {
                            let mut incoming_ids = HashSet::with_capacity(command.items.len());
                            let incoming = command
                                .items
                                .into_iter()
                                .map(|item| {
                                    if !incoming_ids.insert(item.id.clone()) {
                                        return Err(DbErr::Custom(format!(
                                            "duplicated {} game data id in {}: {}",
                                            $game_id, command.collection, item.id
                                        )));
                                    }
                                    Ok($entity::Model {
                                        collection: command.collection.clone(),
                                        id: item.id,
                                        name: item.name,
                                        icon: item.icon,
                                        summary: item.summary,
                                        detail: item.detail,
                                        assets: item.assets,
                                        raw_data: item.raw_data,
                                        source_hash: item.source_hash,
                                    })
                                })
                                .collect::<Result<Vec<_>, DbErr>>()?;
                            let existing = $entity::Entity::find()
                                .filter($entity::Column::Collection.eq(&command.collection))
                                .all(txn)
                                .await?;
                            let existing_by_id = existing
                                .iter()
                                .map(|row| (row.id.as_str(), row))
                                .collect::<HashMap<_, _>>();
                            let created = incoming
                                .iter()
                                .filter(|row| !existing_by_id.contains_key(row.id.as_str()))
                                .count() as u64;
                            let updated = incoming.len() as u64 - created;
                            let changed = existing.len() != incoming.len()
                                || incoming.iter().any(|row| {
                                    existing_by_id.get(row.id.as_str()).copied() != Some(row)
                                });

                            for chunk in incoming.chunks(50) {
                                $entity::Entity::insert_many(
                                    chunk
                                        .iter()
                                        .cloned()
                                        .map(IntoActiveModel::into_active_model),
                                )
                                .on_conflict(upsert())
                                .exec(txn)
                                .await?;
                            }

                            let stale_ids = existing
                                .into_iter()
                                .filter(|row| !incoming_ids.contains(&row.id))
                                .map(|row| row.id)
                                .collect::<Vec<_>>();
                            let deleted = stale_ids.len() as u64;
                            if !stale_ids.is_empty() {
                                if command.collection == "character" {
                                    $character_link::Entity::delete_many()
                                        .filter(
                                            $character_link::Column::CharacterId
                                                .is_in(stale_ids.iter().cloned()),
                                        )
                                        .exec(txn)
                                        .await?;
                                }
                                $entity::Entity::delete_many()
                                    .filter($entity::Column::Collection.eq(&command.collection))
                                    .filter($entity::Column::Id.is_in(stale_ids))
                                    .exec(txn)
                                    .await?;
                            }

                            let total = incoming.len() as u64;
                            audit::insert(
                                txn,
                                &command.audit,
                                concat!("game_data.", $game_id, ".sync"),
                                Some("game_data_collection"),
                                Some(format!("{}:{}", $game_id, command.collection)),
                                json!({
                                    "created": created,
                                    "updated": updated,
                                    "deleted": deleted,
                                    "changed": changed,
                                    "total": total,
                                }),
                            )
                            .await?;

                            Ok(SyncGameDataCollectionResult {
                                created,
                                updated,
                                deleted,
                                changed,
                                total,
                            })
                        })
                    })
                    .await
                    .map_err(transaction_error)
            }

            pub async fn update(
                db: &Db,
                command: UpdateGameDataCollectionCommand,
            ) -> Result<SyncGameDataCollectionResult, DbError> {
                db.conn()
                    .transaction::<_, SyncGameDataCollectionResult, DbErr>(|txn| {
                        Box::pin(async move {
                            let mut incoming_ids = HashSet::with_capacity(command.items.len());
                            let incoming = command
                                .items
                                .into_iter()
                                .map(|item| {
                                    if !incoming_ids.insert(item.id.clone()) {
                                        return Err(DbErr::Custom(format!(
                                            "duplicated {} game data id in {}: {}",
                                            $game_id, command.collection, item.id
                                        )));
                                    }
                                    Ok($entity::Model {
                                        collection: command.collection.clone(),
                                        id: item.id,
                                        name: item.name,
                                        icon: item.icon,
                                        summary: item.summary,
                                        detail: item.detail,
                                        assets: item.assets,
                                        raw_data: item.raw_data,
                                        source_hash: item.source_hash,
                                    })
                                })
                                .collect::<Result<Vec<_>, DbErr>>()?;
                            let removed_ids = command
                                .removed_ids
                                .into_iter()
                                .collect::<HashSet<_>>();
                            if let Some(id) = incoming_ids.intersection(&removed_ids).next() {
                                return Err(DbErr::Custom(format!(
                                    "game data id cannot be updated and removed together: {id}"
                                )));
                            }

                            let mut existing_ids = HashSet::new();
                            for chunk in incoming.chunks(400) {
                                existing_ids.extend(
                                    $entity::Entity::find()
                                        .select_only()
                                        .column($entity::Column::Id)
                                        .filter(
                                            $entity::Column::Collection.eq(&command.collection),
                                        )
                                        .filter(
                                            $entity::Column::Id
                                                .is_in(chunk.iter().map(|row| row.id.clone())),
                                        )
                                        .into_tuple::<String>()
                                        .all(txn)
                                        .await?,
                                );
                            }
                            let created = incoming_ids
                                .iter()
                                .filter(|id| !existing_ids.contains(*id))
                                .count() as u64;
                            let updated = incoming.len() as u64 - created;

                            for chunk in incoming.chunks(50) {
                                $entity::Entity::insert_many(
                                    chunk
                                        .iter()
                                        .cloned()
                                        .map(IntoActiveModel::into_active_model),
                                )
                                .on_conflict(upsert())
                                .exec(txn)
                                .await?;
                            }

                            let removed_ids = removed_ids.into_iter().collect::<Vec<_>>();
                            let mut deleted = 0_u64;
                            for chunk in removed_ids.chunks(400) {
                                if command.collection == "character" {
                                    $character_link::Entity::delete_many()
                                        .filter(
                                            $character_link::Column::CharacterId
                                                .is_in(chunk.iter().cloned()),
                                        )
                                        .exec(txn)
                                        .await?;
                                }
                                deleted += $entity::Entity::delete_many()
                                    .filter($entity::Column::Collection.eq(&command.collection))
                                    .filter($entity::Column::Id.is_in(chunk.iter().cloned()))
                                    .exec(txn)
                                    .await?
                                    .rows_affected;
                            }

                            let total = $entity::Entity::find()
                                .filter($entity::Column::Collection.eq(&command.collection))
                                .count(txn)
                                .await?;
                            let changed = !incoming.is_empty() || deleted > 0;
                            audit::insert(
                                txn,
                                &command.audit,
                                concat!("game_data.", $game_id, ".update"),
                                Some("game_data_collection"),
                                Some(format!("{}:{}", $game_id, command.collection)),
                                json!({
                                    "created": created,
                                    "updated": updated,
                                    "deleted": deleted,
                                    "changed": changed,
                                    "total": total,
                                }),
                            )
                            .await?;

                            Ok(SyncGameDataCollectionResult {
                                created,
                                updated,
                                deleted,
                                changed,
                                total,
                            })
                        })
                    })
                    .await
                    .map_err(transaction_error)
            }

            fn upsert() -> OnConflict {
                OnConflict::columns([$entity::Column::Collection, $entity::Column::Id])
                    .update_columns([
                        $entity::Column::Name,
                        $entity::Column::Icon,
                        $entity::Column::Summary,
                        $entity::Column::Detail,
                        $entity::Column::Assets,
                        $entity::Column::RawData,
                        $entity::Column::SourceHash,
                    ])
                    .to_owned()
            }

            impl From<$entity::Model> for GameDataEntry {
                fn from(value: $entity::Model) -> Self {
                    Self {
                        collection: value.collection,
                        id: value.id,
                        name: value.name,
                        icon: value.icon,
                        summary: value.summary,
                        detail: value.detail,
                        assets: value.assets,
                        raw_data: value.raw_data,
                        source_hash: value.source_hash,
                    }
                }
            }

            fn transaction_error(error: TransactionError<DbErr>) -> DbError {
                match error {
                    TransactionError::Connection(error)
                    | TransactionError::Transaction(error) => DbError::Query(error),
                }
            }
        }
    };
}

game_data_repository!(ys, ys_game_data, ys_news_characters_link, "ys");
game_data_repository!(sr, sr_game_data, sr_news_characters_link, "sr");
game_data_repository!(zzz, zzz_game_data, zzz_news_characters_link, "zzz");

pub async fn list_collections(db: &Db, game_id: &str) -> Result<Vec<GameDataCollection>, DbError> {
    match game_id {
        "ys" => ys::list_collections(db).await,
        "sr" => sr::list_collections(db).await,
        "zzz" => zzz::list_collections(db).await,
        _ => Ok(Vec::new()),
    }
}

pub async fn list(
    db: &Db,
    filter: GameDataListFilter,
) -> Result<(u64, Vec<GameDataEntry>), DbError> {
    if let Some(collection_filter) = filter.collection_filter.clone() {
        return match collection_filter {
            GameDataCollectionFilter::YsCharacter(filter) => {
                crate::repositories::characters::list_ys_entries(db, filter).await
            }
            GameDataCollectionFilter::SrCharacter(filter) => {
                crate::repositories::characters::list_sr_entries(db, filter).await
            }
            GameDataCollectionFilter::ZzzCharacter(filter) => {
                crate::repositories::characters::list_zzz_entries(db, filter).await
            }
        };
    }
    match filter.game_id.as_str() {
        "ys" => ys::list(db, filter).await,
        "sr" => sr::list(db, filter).await,
        "zzz" => zzz::list(db, filter).await,
        _ => Ok((0, Vec::new())),
    }
}

pub async fn find(
    db: &Db,
    game_id: &str,
    collection: &str,
    id: &str,
) -> Result<Option<GameDataEntry>, DbError> {
    match game_id {
        "ys" => ys::find(db, collection, id).await,
        "sr" => sr::find(db, collection, id).await,
        "zzz" => zzz::find(db, collection, id).await,
        _ => Ok(None),
    }
}

pub async fn list_raw(
    db: &Db,
    filter: ListGameDataRawFilter,
) -> Result<(u64, Vec<GameDataRawItem>), DbError> {
    match filter.game_id.as_str() {
        "ys" => ys::list_raw(db, filter).await,
        "sr" => sr::list_raw(db, filter).await,
        "zzz" => zzz::list_raw(db, filter).await,
        _ => Ok((0, Vec::new())),
    }
}

pub async fn sync(
    db: &Db,
    command: SyncGameDataCollectionCommand,
) -> Result<SyncGameDataCollectionResult, DbError> {
    match command.game_id.as_str() {
        "ys" => ys::sync(db, command).await,
        "sr" => sr::sync(db, command).await,
        "zzz" => zzz::sync(db, command).await,
        game_id => Err(DbError::Query(sea_orm::DbErr::Custom(format!(
            "game data is not supported for game {game_id}"
        )))),
    }
}

pub async fn update(
    db: &Db,
    command: UpdateGameDataCollectionCommand,
) -> Result<SyncGameDataCollectionResult, DbError> {
    match command.game_id.as_str() {
        "ys" => ys::update(db, command).await,
        "sr" => sr::update(db, command).await,
        "zzz" => zzz::update(db, command).await,
        game_id => Err(DbError::Query(sea_orm::DbErr::Custom(format!(
            "game data is not supported for game {game_id}"
        )))),
    }
}
