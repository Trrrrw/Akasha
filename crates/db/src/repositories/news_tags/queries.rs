use std::collections::HashMap;

use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait, RelationTrait,
    sea_query::{Expr, Func},
};

use crate::{
    Db, DbError,
    entities::{news, news_tags, news_tags_link},
    models::NewsCount,
    repositories::news::{UNTAGGED_TAG_FILTER, recent_by_tag, recent_untagged},
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

    let mut projections = Vec::with_capacity(rows.len());
    for row in rows {
        let recent = recent_by_tag(db, game_id, source_id, &row.name).await?;

        projections.push(NewsTagProjection {
            news_count: stats_by_name.get(&row.name).copied().unwrap_or_default(),
            name: row.name,
            index: row.index,
            group: row.group,
            group_index: row.group_index,
            recent,
        });
    }

    projections.push(NewsTagProjection {
        name: UNTAGGED_TAG_FILTER.to_owned(),
        index: 0,
        group: Some(UNTAGGED_TAG_FILTER.to_owned()),
        group_index: None,
        news_count: untagged_news_count(db, game_id, source_id).await?,
        recent: recent_untagged(db, game_id, source_id).await?,
    });

    Ok(projections)
}

async fn untagged_news_count(
    db: &Db,
    game_id: &str,
    source_id: &str,
) -> Result<NewsCount, DbError> {
    let tagged_news_ids = news_tags_link::Entity::find()
        .select_only()
        .column(news_tags_link::Column::NewsId)
        .filter(news_tags_link::Column::GameId.eq(game_id))
        .filter(news_tags_link::Column::SourceId.eq(source_id))
        .into_query();

    let (total, video, article) = news::Entity::find()
        .select_only()
        .column_as(news::Column::Id.count(), "total")
        .expr_as(
            Func::sum(Expr::case(news::Column::NewsType.eq(news::NewsType::Video), 1).finally(0)),
            "video",
        )
        .expr_as(
            Func::sum(Expr::case(news::Column::NewsType.eq(news::NewsType::Article), 1).finally(0)),
            "article",
        )
        .filter(news::Column::GameId.eq(game_id))
        .filter(news::Column::SourceId.eq(source_id))
        .filter(news::Column::Id.not_in_subquery(tagged_news_ids))
        .into_tuple::<(i64, Option<i64>, Option<i64>)>()
        .one(db.conn())
        .await
        .map_err(DbError::Query)?
        .unwrap_or_default();

    Ok(NewsCount {
        total: total as u64,
        video: video.unwrap_or_default() as u64,
        article: article.unwrap_or_default() as u64,
    })
}

async fn tag_news_stats(
    db: &Db,
    game_id: &str,
    source_id: &str,
) -> Result<HashMap<String, NewsCount>, DbError> {
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
        .join(
            sea_orm::JoinType::InnerJoin,
            news_tags_link::Relation::News.def(),
        )
        .filter(news_tags_link::Column::GameId.eq(game_id))
        .filter(news_tags_link::Column::SourceId.eq(source_id))
        .group_by(news_tags_link::Column::Name)
        .into_tuple::<(String, i64, i64, i64)>()
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    Ok(rows
        .into_iter()
        .map(|(name, total, video, article)| {
            (
                name,
                NewsCount {
                    total: total as u64,
                    video: video as u64,
                    article: article as u64,
                },
            )
        })
        .collect())
}
