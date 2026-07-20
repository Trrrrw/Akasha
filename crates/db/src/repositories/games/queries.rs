use std::collections::HashMap;

use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect,
    entity::prelude::DateTimeWithTimeZone,
    sea_query::{Expr, Func},
};

use crate::{
    Db, DbError,
    entities::{games, news},
    models::NewsStats,
};

use super::projections::GameSummary;

/// 列出所有游戏
pub async fn list(db: &Db) -> Result<Vec<GameSummary>, DbError> {
    let rows = games::Entity::find()
        .order_by(games::Column::Index, sea_orm::Order::Asc)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;
    let news_counts = news_stats(db, None).await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let counts = news_counts
                .get(row.id.as_str())
                .copied()
                .unwrap_or_default();
            into_summary(row, counts)
        })
        .collect())
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
    let counts = news_stats(db, Some(&row.id))
        .await?
        .remove(row.id.as_str())
        .unwrap_or_default();
    Ok(Some(into_summary(row, counts)))
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
struct NewsStatsRow {
    game_id: String,
    total: i64,
    video: i64,
    article: i64,
    latest_publish_time: Option<DateTimeWithTimeZone>,
    latest_video_publish_time: Option<DateTimeWithTimeZone>,
}

async fn news_stats(db: &Db, game_id: Option<&str>) -> Result<HashMap<String, NewsStats>, DbError> {
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
        .column_as(news::Column::PublishTime.max(), "latest_publish_time")
        .expr_as(
            Func::max(
                Expr::case(
                    news::Column::NewsType.eq(news::NewsType::Video),
                    Expr::col(news::Column::PublishTime),
                )
                .finally(Expr::null()),
            ),
            "latest_video_publish_time",
        )
        .group_by(news::Column::GameId);
    if let Some(game_id) = game_id {
        query = query.filter(news::Column::GameId.eq(game_id));
    }
    let rows = query
        .into_model::<NewsStatsRow>()
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.game_id,
                NewsStats {
                    total: row.total as u64,
                    video: row.video as u64,
                    article: row.article as u64,
                    latest_publish_time: row.latest_publish_time,
                    latest_video_publish_time: row.latest_video_publish_time,
                },
            )
        })
        .collect())
}

fn into_summary(row: games::Model, counts: NewsStats) -> GameSummary {
    GameSummary {
        id: row.id,
        name_en: row.name_en,
        name_zh: row.name_zh,
        index: row.index,
        cover: row.cover,
        icon: row.icon,
        news_stats: counts,
    }
}
