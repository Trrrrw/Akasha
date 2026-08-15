use akasha_application::characters::{SyncCharacterItem, SyncCharactersCommand};
use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    http::{
        error::AppError,
        extractors::{AuditRequest, DataWriteActor},
    },
    state::AppState,
};

/// 同步一个游戏角色目录的 HTTP 请求体
#[derive(Deserialize)]
pub(crate) struct SyncCharactersRequest {
    game_id: String,
    items: Vec<SyncCharacterRequestItem>,
    audit: Option<AuditRequest>,
}

/// 角色目录同步请求中的单个角色
#[derive(Deserialize)]
pub(crate) struct SyncCharacterRequestItem {
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

/// 角色目录同步后的数量统计响应
#[derive(Serialize)]
pub(crate) struct SyncCharactersResponse {
    created: u64,
    updated: u64,
    deleted: u64,
    changed: bool,
    total: u64,
}

/// 同步一个游戏的完整角色目录
pub(crate) async fn sync(
    actor: DataWriteActor,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SyncCharactersRequest>,
) -> Result<Json<SyncCharactersResponse>, AppError> {
    let audit = actor.audit_context(body.audit.unwrap_or_default(), &headers);
    tracing::info!(
        actor = %actor.label(),
        game_id = %body.game_id,
        character_count = body.items.len(),
        "syncing characters"
    );

    let result = state
        .application()
        .sync_characters(SyncCharactersCommand {
            game_id: body.game_id,
            audit,
            items: body
                .items
                .into_iter()
                .map(|item| SyncCharacterItem {
                    id: item.id,
                    item_id: item.item_id,
                    name: item.name,
                    description: item.description,
                    gender: item.gender,
                    birthday_month: item.birthday_month,
                    birthday_day: item.birthday_day,
                    voice_actor: item.cv,
                    extra: item.extra,
                })
                .collect(),
        })
        .await?;

    Ok(Json(SyncCharactersResponse {
        created: result.created,
        updated: result.updated,
        deleted: result.deleted,
        changed: result.changed,
        total: result.total,
    }))
}
