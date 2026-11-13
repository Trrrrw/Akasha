use crate::{
    http::{
        error::AppError,
        path::require_game,
        response::{ErrorResponse, PageResponse, public_asset_url},
    },
    state::AppState,
};
use akasha_application::characters::{
    SrCharacter, SrCharacterListFilter, YsCharacter, YsCharacterListFilter, ZzzCharacter,
    ZzzCharacterListFilter,
};
use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// 角色目录分页参数
#[derive(Default, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(super) struct CharacterQuery {
    /// 每页数量，默认 100，范围 1 到 100
    limit: Option<u64>,
    /// 分页偏移，默认 0
    offset: Option<u64>,
}

/// Ys 角色公共响应，字段仅在本游戏的资源中扩展
#[derive(Serialize, ToSchema)]
pub(super) struct YsCharacterResponse {
    id: String,
    name: String,
    icon: Option<String>,
    birthday_month: Option<i16>,
    birthday_day: Option<i16>,
}
impl YsCharacterResponse {
    fn from_character(item: YsCharacter, asset_base: &str) -> Self {
        Self {
            id: item.id,
            name: item.name,
            icon: public_asset_url(
                asset_base,
                item.icon_url
                    .filter(|v| v.starts_with("/assets/game-data/")),
            ),
            birthday_month: item.birthday_month,
            birthday_day: item.birthday_day,
        }
    }
}

/// Sr 角色公共响应，字段仅在本游戏的资源中扩展
#[derive(Serialize, ToSchema)]
pub(super) struct SrCharacterResponse {
    id: String,
    name: String,
    icon: Option<String>,
    birthday_month: Option<i16>,
    birthday_day: Option<i16>,
}
impl SrCharacterResponse {
    fn from_character(item: SrCharacter, asset_base: &str) -> Self {
        Self {
            id: item.id,
            name: item.name,
            icon: public_asset_url(
                asset_base,
                item.icon_url
                    .filter(|v| v.starts_with("/assets/game-data/")),
            ),
            birthday_month: item.birthday_month,
            birthday_day: item.birthday_day,
        }
    }
}

/// Zzz 角色公共响应，字段仅在本游戏的资源中扩展
#[derive(Serialize, ToSchema)]
pub(super) struct ZzzCharacterResponse {
    id: String,
    name: String,
    icon: Option<String>,
    birthday_month: Option<i16>,
    birthday_day: Option<i16>,
}
impl ZzzCharacterResponse {
    fn from_character(item: ZzzCharacter, asset_base: &str) -> Self {
        Self {
            id: item.id,
            name: item.name,
            icon: public_asset_url(
                asset_base,
                item.icon_url
                    .filter(|v| v.starts_with("/assets/game-data/")),
            ),
            birthday_month: item.birthday_month,
            birthday_day: item.birthday_day,
        }
    }
}

/// 每个游戏注册独立响应 schema，共用 HTTP 分页流程
macro_rules! character_endpoint {
    ($function:ident, $path:literal, $game:literal, $model:ident, $filter:ident, $method:ident) => {
        #[utoipa::path(
                            get,
                            path = $path,
                            tag = "Game Data",
                            summary = "获取角色目录",
                            description = "返回本游戏的角色目录，支持分页；尚无资料时返回空列表",
                            params(CharacterQuery),
                            responses(
                                (status = 200, body = PageResponse<$model>),
                                (status = 400, body = ErrorResponse),
                                (status = 404, body = ErrorResponse),
                                (status = 500, body = ErrorResponse)
                            )
                        )]
        pub(super) async fn $function(
            State(state): State<AppState>,
            Query(query): Query<CharacterQuery>,
        ) -> Result<Json<PageResponse<$model>>, AppError> {
            require_game(&state, $game).await?;
            let limit = query.limit.unwrap_or(100);
            if !(1..=100).contains(&limit) {
                return Err(AppError::BadRequest(
                    "character limit must be between 1 and 100".into(),
                ));
            }
            let offset = query.offset.unwrap_or(0);
            let (total, items) = state
                .application()
                .$method($filter {
                    birthday_only: false,
                    limit,
                    offset,
                })
                .await?;
            Ok(Json(PageResponse {
                total,
                limit,
                offset,
                meta: (),
                items: items
                    .into_iter()
                    .map(|item| $model::from_character(item, &state.config().asset_base_url))
                    .collect(),
            }))
        }
    };
}
character_endpoint!(
    ys_characters,
    "/games/ys/characters",
    "ys",
    YsCharacterResponse,
    YsCharacterListFilter,
    list_ys_characters
);
character_endpoint!(
    sr_characters,
    "/games/sr/characters",
    "sr",
    SrCharacterResponse,
    SrCharacterListFilter,
    list_sr_characters
);
character_endpoint!(
    zzz_characters,
    "/games/zzz/characters",
    "zzz",
    ZzzCharacterResponse,
    ZzzCharacterListFilter,
    list_zzz_characters
);

pub(super) fn validate_game(game_id: &str) -> Result<(), AppError> {
    if matches!(game_id, "ys" | "sr" | "zzz") {
        Ok(())
    } else {
        Err(AppError::NotFound(format!(
            "game data is not available for game {game_id}"
        )))
    }
}

pub(super) fn validate_collection(collection: &str) -> Result<(), AppError> {
    if matches!(collection, "achievement" | "character") {
        Ok(())
    } else {
        Err(AppError::NotFound(format!(
            "unsupported collection: {collection}"
        )))
    }
}
