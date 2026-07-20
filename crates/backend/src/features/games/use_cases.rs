use akasha_db::{
    Db, DbError,
    repositories::games::{self, GameSummary},
};

pub(super) async fn list(db: &Db) -> Result<Vec<GameSummary>, DbError> {
    games::list(db).await
}

pub(super) async fn detail(db: &Db, game_id: &str) -> Result<Option<GameSummary>, DbError> {
    games::find_by_id(db, game_id).await
}
