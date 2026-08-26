use serde::Deserialize;
use utoipa::IntoParams;

use crate::{http::error::AppError, state::AppState};

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub(crate) struct GamePath {
    /// 游戏 ID
    pub game_id: String,
}

/// 确认路径中的游戏资源存在
pub(crate) async fn require_game(state: &AppState, game_id: &str) -> Result<(), AppError> {
    if state.application().find_game(game_id).await?.is_some() {
        Ok(())
    } else {
        Err(AppError::NotFound(format!("game {game_id} not found")))
    }
}

/// 确认游戏下的新闻来源资源存在
pub(crate) async fn require_news_source(
    state: &AppState,
    game_id: &str,
    source_id: &str,
) -> Result<(), AppError> {
    if source_id.trim().is_empty() {
        return Err(AppError::BadRequest(
            "source_id must not be empty".to_owned(),
        ));
    }
    require_game(state, game_id).await?;
    if state
        .application()
        .list_news_sources(game_id)
        .await?
        .into_iter()
        .any(|source| source.id == source_id)
    {
        Ok(())
    } else {
        Err(AppError::NotFound(format!(
            "news source {source_id} not found in {game_id}"
        )))
    }
}

/// 确认游戏下的游戏数据集合已经有可读数据
pub(crate) async fn require_game_data_collection(
    state: &AppState,
    game_id: &str,
    collection: &str,
) -> Result<(), AppError> {
    require_game(state, game_id).await?;
    if state
        .application()
        .list_game_data_collections(game_id)
        .await?
        .into_iter()
        .any(|item| item.id == collection)
    {
        Ok(())
    } else {
        Err(AppError::NotFound(format!(
            "game data collection {collection} not found for {game_id}"
        )))
    }
}
