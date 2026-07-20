use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    http::{error::AppError, extractors::DataWriteActor},
    state::AppState,
};

#[derive(Deserialize)]
pub(crate) struct SyncCharactersRequest {
    game_id: String,
    items: Vec<SyncCharacterItem>,
}

#[derive(Deserialize)]
pub(crate) struct SyncCharacterItem {
    id: String,
    item_id: String,
    name: String,
    description: Option<String>,
    gender: Option<String>,
    birthday_month: Option<i16>,
    birthday_day: Option<i16>,
    cv: Option<String>,
    extra: Value,
}

#[derive(Serialize)]
pub(crate) struct SyncCharactersResponse {
    created: u64,
    updated: u64,
    total: u64,
}

pub(crate) async fn sync(
    actor: DataWriteActor,
    State(state): State<AppState>,
    Json(body): Json<SyncCharactersRequest>,
) -> Result<Json<SyncCharactersResponse>, AppError> {
    tracing::info!(actor = %actor.label(), game_id = %body.game_id, chars = body.items.len(), "syncing chars");

    let result = akasha_db::repositories::characters::sync_chars(
        state.db(),
        akasha_db::repositories::characters::SyncCharsInput {
            game_id: body.game_id,
            items: body
                .items
                .into_iter()
                .map(|item| akasha_db::repositories::characters::SyncCharInput {
                    id: item.id,
                    item_id: item.item_id,
                    name: item.name,
                    description: item.description,
                    gender: item.gender,
                    birthday_month: item.birthday_month,
                    birthday_day: item.birthday_day,
                    cv: item.cv,
                    extra: item.extra,
                })
                .collect(),
        },
    )
    .await
    .map_err(|err| AppError::Internal(err.into()))?;

    Ok(Json(SyncCharactersResponse {
        created: result.created,
        updated: result.updated,
        total: result.total,
    }))
}
