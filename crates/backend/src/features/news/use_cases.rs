use akasha_db::{
    Db, DbError,
    repositories::{
        games,
        news::{self, projections::NewsSourceProjection},
        news_tags::{self, NewsTagProjection},
    },
};

pub(crate) async fn list_sources(
    db: &Db,
    game_id: &str,
) -> Result<Vec<NewsSourceProjection>, DbError> {
    news::queries::list_sources(db, game_id).await
}

pub(crate) async fn list_tags(
    db: &Db,
    game_id: &str,
    source_id: &str,
) -> Result<Vec<NewsTagProjection>, DbError> {
    news_tags::list_tags(db, game_id, source_id).await
}

pub(crate) async fn list(
    db: &Db,
    filter: news::ListNewsFilter,
) -> Result<(u64, Vec<news::NewsSummary>, Option<String>), DbError> {
    let game_id = filter.game_id.clone();
    let (total, rows) = news::list(db, filter).await?;
    let game_cover = games::find_cover_by_id(db, &game_id).await?;

    Ok((total, rows, game_cover))
}

pub(crate) async fn detail(
    db: &Db,
    game_id: &str,
    source_id: &str,
    news_id: &str,
) -> Result<Option<(news::NewsSummary, Option<String>)>, DbError> {
    let Some(news) = news::find_by_id(db, game_id, source_id, news_id).await? else {
        return Ok(None);
    };
    let game_cover = games::find_cover_by_id(db, game_id).await?;

    Ok(Some((news, game_cover)))
}
