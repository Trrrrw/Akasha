use std::path::{Component, Path, PathBuf};

use akasha_application::game_data::{
    GameDataEntry, ListGameDataRawFilter, SyncGameDataCollectionCommand,
    SyncGameDataCollectionResult, UpdateGameDataCollectionCommand,
};
use axum::{
    Json,
    body::Bytes,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    features::game_data::endpoints::{validate_collection, validate_game},
    http::{
        error::AppError,
        extractors::{AuditRequest, DataWriteActor},
        path::require_game,
    },
    state::AppState,
};

#[derive(Deserialize)]
pub(crate) struct SyncGameDataCollectionRequest {
    items: Vec<GameDataEntryRequest>,
    audit: Option<AuditRequest>,
}

#[derive(Deserialize)]
pub(crate) struct GameDataEntryRequest {
    id: String,
    name: Option<String>,
    icon: Option<String>,
    summary: Value,
    detail: Option<Value>,
    assets: Value,
    raw_data: Option<Value>,
    source_hash: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct UpdateGameDataCollectionRequest {
    items: Vec<GameDataEntryRequest>,
    removed_ids: Vec<String>,
    audit: Option<AuditRequest>,
}

#[derive(Deserialize)]
pub(crate) struct GameDataRawQuery {
    after_id: Option<String>,
    limit: Option<u64>,
    include_raw_data: Option<bool>,
}

#[derive(Serialize)]
pub(crate) struct GameDataRawPageResponse {
    total: u64,
    limit: u64,
    items: Vec<GameDataRawItemResponse>,
    next_cursor: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct GameDataRawItemResponse {
    id: String,
    raw_data: Option<Value>,
    source_hash: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct SyncGameDataCollectionResponse {
    created: u64,
    updated: u64,
    deleted: u64,
    changed: bool,
    total: u64,
}

pub(crate) async fn sync_collection(
    actor: DataWriteActor,
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((game_id, collection)): AxumPath<(String, String)>,
    Json(body): Json<SyncGameDataCollectionRequest>,
) -> Result<Json<SyncGameDataCollectionResponse>, AppError> {
    validate_game(&game_id)?;
    require_game(&state, &game_id).await?;
    validate_collection(&collection)?;
    if collection == "character" && body.items.is_empty() {
        return Err(AppError::BadRequest(
            "character collection must not be empty".to_owned(),
        ));
    }
    if body.items.iter().any(|item| {
        !valid_optional_asset_path(item.icon.as_deref())
            || contains_external_url(&item.summary)
            || item.detail.as_ref().is_some_and(contains_external_url)
            || contains_external_url(&item.assets)
    }) {
        return Err(AppError::BadRequest(
            "game data resources must use backend asset paths".to_owned(),
        ));
    }
    let audit = actor.audit_context(body.audit.unwrap_or_default(), &headers);
    let result = state
        .application()
        .sync_game_data_collection(SyncGameDataCollectionCommand {
            game_id,
            collection: collection.clone(),
            items: body
                .items
                .into_iter()
                .map(|item| GameDataEntry {
                    collection: collection.clone(),
                    id: item.id,
                    name: item.name,
                    icon: item.icon,
                    summary: item.summary,
                    detail: item.detail,
                    assets: item.assets,
                    raw_data: item.raw_data,
                    source_hash: item.source_hash,
                })
                .collect(),
            audit,
        })
        .await?;
    Ok(Json(result.into()))
}

pub(crate) async fn list_raw(
    actor: DataWriteActor,
    State(state): State<AppState>,
    AxumPath((game_id, collection)): AxumPath<(String, String)>,
    Query(query): Query<GameDataRawQuery>,
) -> Result<Json<GameDataRawPageResponse>, AppError> {
    validate_game(&game_id)?;
    require_game(&state, &game_id).await?;
    validate_collection(&collection)?;
    let limit = query.limit.unwrap_or(500).clamp(1, 1_000);
    tracing::debug!(actor = %actor.label(), game_id, collection, "listing raw game data");
    let (total, items) = state
        .application()
        .list_game_data_raw(ListGameDataRawFilter {
            game_id,
            collection,
            after_id: query.after_id,
            include_raw_data: query.include_raw_data.unwrap_or(false),
            limit,
        })
        .await?;
    let next_cursor = items.last().map(|item| item.id.clone());
    let include_raw_data = query.include_raw_data.unwrap_or(false);
    Ok(Json(GameDataRawPageResponse {
        total,
        limit,
        next_cursor,
        items: items
            .into_iter()
            .map(|item| GameDataRawItemResponse {
                id: item.id,
                raw_data: include_raw_data.then_some(item.raw_data).flatten(),
                source_hash: item.source_hash,
            })
            .collect(),
    }))
}

pub(crate) async fn update_collection(
    actor: DataWriteActor,
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((game_id, collection)): AxumPath<(String, String)>,
    Json(body): Json<UpdateGameDataCollectionRequest>,
) -> Result<Json<SyncGameDataCollectionResponse>, AppError> {
    validate_game(&game_id)?;
    require_game(&state, &game_id).await?;
    validate_collection(&collection)?;
    validate_entries(&body.items)?;
    let audit = actor.audit_context(body.audit.unwrap_or_default(), &headers);
    let result = state
        .application()
        .update_game_data_collection(UpdateGameDataCollectionCommand {
            game_id,
            collection: collection.clone(),
            items: body
                .items
                .into_iter()
                .map(|item| GameDataEntry {
                    collection: collection.clone(),
                    id: item.id,
                    name: item.name,
                    icon: item.icon,
                    summary: item.summary,
                    detail: item.detail,
                    assets: item.assets,
                    raw_data: item.raw_data,
                    source_hash: item.source_hash,
                })
                .collect(),
            removed_ids: body.removed_ids,
            audit,
        })
        .await?;
    Ok(Json(result.into()))
}

pub(crate) async fn upload_asset(
    actor: DataWriteActor,
    State(state): State<AppState>,
    AxumPath((game_id, path)): AxumPath<(String, String)>,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    validate_game(&game_id)?;
    require_game(&state, &game_id).await?;
    let relative = safe_asset_path(&path)?;
    let root = state.config().game_data_asset_dir.join(&game_id);
    let target = root.join(relative);
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| AppError::Internal(error.into()))?;
    }
    tokio::fs::write(&target, body)
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    tracing::debug!(actor = %actor.label(), game_id, path = %target.display(), "stored game data asset");
    Ok(StatusCode::NO_CONTENT)
}

fn valid_optional_asset_path(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.starts_with("/assets/game-data/"))
}

fn validate_entries(items: &[GameDataEntryRequest]) -> Result<(), AppError> {
    if items.iter().any(|item| {
        !valid_optional_asset_path(item.icon.as_deref())
            || contains_external_url(&item.summary)
            || item.detail.as_ref().is_some_and(contains_external_url)
            || contains_external_url(&item.assets)
    }) {
        return Err(AppError::BadRequest(
            "game data resources must use backend asset paths".to_owned(),
        ));
    }
    Ok(())
}

fn contains_external_url(value: &Value) -> bool {
    match value {
        Value::String(value) => {
            let value = value.to_ascii_lowercase();
            value.starts_with("http://") || value.starts_with("https://") || value.starts_with("//")
        }
        Value::Array(values) => values.iter().any(contains_external_url),
        Value::Object(values) => values.values().any(contains_external_url),
        _ => false,
    }
}

fn safe_asset_path(value: &str) -> Result<PathBuf, AppError> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(AppError::BadRequest(
            "invalid game data asset path".to_owned(),
        ));
    }
    Ok(path.to_owned())
}

impl From<SyncGameDataCollectionResult> for SyncGameDataCollectionResponse {
    fn from(value: SyncGameDataCollectionResult) -> Self {
        Self {
            created: value.created,
            updated: value.updated,
            deleted: value.deleted,
            changed: value.changed,
            total: value.total,
        }
    }
}
