use akasha_application::game_data::{GameDataCollection, GameDataEntry};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::http::response::{public_asset_json, public_asset_url};

/// 一个游戏数据集合的摘要
#[derive(Serialize, ToSchema)]
pub(super) struct GameDataCollectionResponse {
    id: String,
    total: u64,
}

/// 游戏数据列表条目
#[derive(Serialize, ToSchema)]
pub(super) struct GameDataEntryResponse {
    id: String,
    name: Option<String>,
    icon: Option<String>,
    summary: Value,
    assets: Value,
}

/// 游戏数据详情
#[derive(Serialize, ToSchema)]
pub(super) struct GameDataDetailResponse {
    collection: String,
    id: String,
    name: Option<String>,
    icon: Option<String>,
    summary: Value,
    detail: Option<Value>,
    assets: Value,
}

impl From<GameDataCollection> for GameDataCollectionResponse {
    fn from(value: GameDataCollection) -> Self {
        Self {
            id: value.id,
            total: value.total,
        }
    }
}

impl GameDataEntryResponse {
    pub(super) fn from_entry(value: GameDataEntry, asset_base_url: &str) -> Self {
        Self {
            id: value.id,
            name: value.name,
            icon: owned_asset_url(asset_base_url, value.icon),
            summary: public_asset_json(asset_base_url, value.summary),
            assets: public_asset_json(asset_base_url, value.assets),
        }
    }
}

impl GameDataDetailResponse {
    pub(super) fn from_entry(value: GameDataEntry, asset_base_url: &str) -> Self {
        Self {
            collection: value.collection,
            id: value.id,
            name: value.name,
            icon: owned_asset_url(asset_base_url, value.icon),
            summary: public_asset_json(asset_base_url, value.summary),
            detail: value
                .detail
                .map(|detail| public_asset_json(asset_base_url, detail)),
            assets: public_asset_json(asset_base_url, value.assets),
        }
    }
}

fn owned_asset_url(asset_base_url: &str, value: Option<String>) -> Option<String> {
    value
        .filter(|value| value.starts_with("/assets/game-data/"))
        .and_then(|value| public_asset_url(asset_base_url, Some(value)))
}
