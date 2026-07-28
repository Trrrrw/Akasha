use std::collections::HashMap;

use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect,
    sea_query::{Expr, Func},
};

use crate::{
    Db, DbError,
    entities::{games, news},
    models::{NewsCount, RecentNews},
    repositories::news::recent_by_game,
};

use super::projections::GameSummary;

/// 列出所有游戏
pub async fn list(db: &Db) -> Result<Vec<GameSummary>, DbError> {
    let rows = games::Entity::find()
        .order_by(games::Column::Index, sea_orm::Order::Asc)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;
    let news_counts = news_counts(db, None).await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let news_count = news_counts
            .get(row.id.as_str())
            .copied()
            .unwrap_or_default();
        let recent_news = recent_by_game(db, &row.id).await?;
        items.push(into_summary(row, news_count, recent_news));
    }

    Ok(items)
}

/// 获取指定游戏的信息
pub async fn find_by_id(db: &Db, game_id: &str) -> Result<Option<GameSummary>, DbError> {
    let Some(row) = games::Entity::find_by_id(game_id)
        .one(db.conn())
        .await
        .map_err(DbError::Query)?
    else {
        return Ok(None);
    };
    let news_count = news_counts(db, Some(&row.id))
        .await?
        .remove(row.id.as_str())
        .unwrap_or_default();
    let recent_news = recent_by_game(db, &row.id).await?;
    Ok(Some(into_summary(row, news_count, recent_news)))
}

/// 获取指定游戏的封面
pub async fn find_cover_by_id(db: &Db, game_id: &str) -> Result<Option<String>, DbError> {
    games::Entity::find_by_id(game_id)
        .select_only()
        .column(games::Column::Cover)
        .into_tuple::<Option<String>>()
        .one(db.conn())
        .await
        .map_err(DbError::Query)
        .map(Option::flatten)
}

#[derive(Debug, FromQueryResult)]
struct NewsCountRow {
    game_id: String,
    total: i64,
    video: i64,
    article: i64,
}

async fn news_counts(
    db: &Db,
    game_id: Option<&str>,
) -> Result<HashMap<String, NewsCount>, DbError> {
    let mut query = news::Entity::find()
        .select_only()
        .column(news::Column::GameId)
        .column_as(news::Column::Id.count(), "total")
        .expr_as(
            Func::sum(Expr::case(news::Column::NewsType.eq(news::NewsType::Video), 1).finally(0)),
            "video",
        )
        .expr_as(
            Func::sum(Expr::case(news::Column::NewsType.eq(news::NewsType::Article), 1).finally(0)),
            "article",
        )
        .group_by(news::Column::GameId);
    if let Some(game_id) = game_id {
        query = query.filter(news::Column::GameId.eq(game_id));
    }
    let rows = query
        .into_model::<NewsCountRow>()
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.game_id,
                NewsCount {
                    total: row.total as u64,
                    video: row.video as u64,
                    article: row.article as u64,
                },
            )
        })
        .collect())
}

fn into_summary(row: games::Model, news_count: NewsCount, recent_news: RecentNews) -> GameSummary {
    GameSummary {
        id: row.id,
        name_en: row.name_en,
        name_zh: row.name_zh,
        index: row.index,
        cover: row.cover,
        icon: row.icon,
        news_count,
        recent_news,
    }
}
