use std::collections::HashMap;

use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait,
    prelude::DateTimeWithTimeZone,
    sea_query::{Expr, Func},
};

use crate::{
    Db, DbError,
    entities::{news, news_tags, news_tags_link},
    models::NewsStats,
};

use super::projections::NewsTagProjection;

pub async fn list_tags(
    db: &Db,
    game_id: &str,
    source_id: &str,
) -> Result<Vec<NewsTagProjection>, DbError> {
    let rows = news_tags::Entity::find()
        .filter(news_tags::Column::GameId.eq(game_id))
        .filter(news_tags::Column::SourceId.eq(source_id))
        .order_by_asc(news_tags::Column::GroupIndex)
        .order_by_asc(news_tags::Column::Group)
        .order_by(news_tags::Column::Index, sea_orm::Order::Asc)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    let stats_by_name = tag_news_stats(db, game_id, source_id).await?;

    Ok(rows
        .into_iter()
        .map(|row| NewsTagProjection {
            news_stats: stats_by_name.get(&row.name).copied().unwrap_or_default(),
            name: row.name,
            index: row.index,
            group: row.group,
            group_index: row.group_index,
        })
        .collect())
}

async fn tag_news_stats(
    db: &Db,
    game_id: &str,
    source_id: &str,
) -> Result<HashMap<String, NewsStats>, DbError> {
    let rows = news_tags_link::Entity::find()
        .select_only()
        .column(news_tags_link::Column::Name)
        .column_as(news_tags_link::Column::NewsId.count(), "total")
        .expr_as(
            Func::sum(Expr::case(news::Column::NewsType.eq(news::NewsType::Video), 1).finally(0)),
            "video",
        )
        .expr_as(
            Func::sum(Expr::case(news::Column::NewsType.eq(news::NewsType::Article), 1).finally(0)),
            "article",
        )
        .expr_as(news::Column::PublishTime.max(), "latest_publish_time")
        .expr_as(
            Func::max(
                Expr::case(
                    news::Column::NewsType.eq(news::NewsType::Video),
                    Expr::col(news::Column::PublishTime),
                )
                .finally(Expr::val(Option::<DateTimeWithTimeZone>::None)),
            ),
            "latest_video_publish_time",
        )
        .join(
            sea_orm::JoinType::InnerJoin,
            news_tags_link::Relation::News.def(),
        )
        .filter(news_tags_link::Column::GameId.eq(game_id))
        .filter(news_tags_link::Column::SourceId.eq(source_id))
        .group_by(news_tags_link::Column::Name)
        .into_tuple::<(
            String,
            i64,
            i64,
            i64,
            Option<DateTimeWithTimeZone>,
            Option<DateTimeWithTimeZone>,
        )>()
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    Ok(rows
        .into_iter()
        .map(
            |(name, total, video, article, latest_publish_time, latest_video_publish_time)| {
                (
                    name,
                    NewsStats {
                        total: total as u64,
                        video: video as u64,
                        article: article as u64,
                        latest_publish_time,
                        latest_video_publish_time,
                    },
                )
            },
        )
        .collect())
}
