use std::collections::HashMap;

use sea_orm::{
    ActiveEnum, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    QueryTrait, prelude::DateTimeWithTimeZone,
};

use crate::{
    Db, DbError,
    entities::{news, news_sources, news_tags_link},
    models::TitleQuery,
};

use super::projections::{
    ListNewsFilter, NewsSourceProjection, NewsSourceStats, NewsSourceSummary, NewsSummary,
};

/// 列出所有新闻来源
pub async fn list_sources(db: &Db, game_id: &str) -> Result<Vec<NewsSourceProjection>, DbError> {
    let rows = news_sources::Entity::find()
        .filter(news_sources::Column::GameId.eq(game_id))
        .order_by(news_sources::Column::Index, sea_orm::Order::Asc)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    Ok(rows.into_iter().map(NewsSourceProjection::from).collect())
}

/// 获取指定游戏的指定来源信息
pub async fn find_source_by_id(
    db: &Db,
    source_id: &str,
    game_id: &str,
) -> Result<Option<NewsSourceSummary>, DbError> {
    let row = news_sources::Entity::find_by_id((source_id.to_owned(), game_id.to_owned()))
        .one(db.conn())
        .await
        .map_err(DbError::Query)?;

    Ok(row.map(NewsSourceSummary::from))
}

/// 获取指定游戏的指定来源状态
pub async fn source_stats(
    db: &Db,
    source_id: &str,
    game_id: &str,
) -> Result<NewsSourceStats, DbError> {
    let news_query = news::Entity::find()
        .filter(news::Column::GameId.eq(game_id))
        .filter(news::Column::SourceId.eq(source_id));

    let news_count = news_query
        .clone()
        .count(db.conn())
        .await
        .map_err(DbError::Query)?;

    let latest_release_time = news_query
        .select_only()
        .column_as(news::Column::PublishTime.max(), "latest_release_time")
        .into_tuple::<Option<DateTimeWithTimeZone>>()
        .one(db.conn())
        .await
        .map_err(DbError::Query)?
        .flatten();

    Ok(NewsSourceStats {
        news_count,
        latest_release_time,
    })
}

/// 列出新闻
pub async fn list(db: &Db, filter: ListNewsFilter) -> Result<(u64, Vec<NewsSummary>), DbError> {
    let mut query = news::Entity::find()
        .filter(news::Column::GameId.eq(&filter.game_id))
        .filter(news::Column::SourceId.eq(&filter.source_id));

    if let Some(start) = filter.start_publish_time {
        query = query.filter(news::Column::PublishTime.gte(start));
    }

    if let Some(end) = filter.end_publish_time {
        query = query.filter(news::Column::PublishTime.lt(end));
    }

    if let Some(news_type) = filter.news_type {
        let news_type = news::NewsType::try_from_value(&news_type).map_err(DbError::Query)?;
        query = query.filter(news::Column::NewsType.eq(news_type));
    }

    if let Some(tags) = filter.tags.as_ref().filter(|items| !items.is_empty()) {
        let tag_news_ids = news_tags_link::Entity::find()
            .select_only()
            .column(news_tags_link::Column::NewsId)
            .filter(news_tags_link::Column::GameId.eq(&filter.game_id))
            .filter(news_tags_link::Column::SourceId.eq(&filter.source_id))
            .filter(news_tags_link::Column::Name.is_in(tags.iter().cloned()))
            .into_query();

        query = query.filter(news::Column::Id.in_subquery(tag_news_ids));
    }

    if let Some(q) = filter.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        let title_query = TitleQuery::new(q);

        for keyword in title_query.includes {
            query = query.filter(news::Column::Title.contains(&keyword));
        }

        for keyword in title_query.excludes {
            query = query.filter(news::Column::Title.not_like(format!("%{keyword}%")));
        }
    }

    let total = query
        .clone()
        .count(db.conn())
        .await
        .map_err(DbError::Query)?;

    // 在数据库中按请求方向排序后再分页
    let publish_time_order = if filter.reverse {
        sea_orm::Order::Asc
    } else {
        sea_orm::Order::Desc
    };
    let rows = query
        .order_by(news::Column::PublishTime, publish_time_order)
        .limit(filter.limit)
        .offset(filter.offset)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    let news_ids = rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
    let mut tags_map = news_tags_map(db, &filter.game_id, &filter.source_id, &news_ids).await?;

    let items = rows
        .into_iter()
        .map(|row| {
            let tags = tags_map.remove(&row.id).unwrap_or_default();
            into_summary(row, tags)
        })
        .collect();

    Ok((total, items))
}

pub async fn find_by_id(
    db: &Db,
    game_id: &str,
    source_id: &str,
    news_id: &str,
) -> Result<Option<NewsSummary>, DbError> {
    let row =
        news::Entity::find_by_id((game_id.to_owned(), source_id.to_owned(), news_id.to_owned()))
            .one(db.conn())
            .await
            .map_err(DbError::Query)?;

    match row {
        Some(row) => {
            let tags = news_tags_link::Entity::find()
                .select_only()
                .column(news_tags_link::Column::Name)
                .filter(news_tags_link::Column::GameId.eq(game_id))
                .filter(news_tags_link::Column::SourceId.eq(source_id))
                .filter(news_tags_link::Column::NewsId.eq(news_id))
                .into_tuple::<String>()
                .all(db.conn())
                .await
                .map_err(DbError::Query)?;

            Ok(Some(into_summary(row, tags)))
        }
        None => Ok(None),
    }
}

async fn news_tags_map(
    db: &Db,
    game_id: &str,
    source_id: &str,
    news_ids: &[String],
) -> Result<HashMap<String, Vec<String>>, DbError> {
    let rows = news_tags_link::Entity::find()
        .select_only()
        .column(news_tags_link::Column::NewsId)
        .column(news_tags_link::Column::Name)
        .filter(news_tags_link::Column::GameId.eq(game_id))
        .filter(news_tags_link::Column::SourceId.eq(source_id))
        .filter(news_tags_link::Column::NewsId.is_in(news_ids.iter().cloned()))
        .into_tuple::<(String, String)>()
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (news_id, tag) in rows {
        map.entry(news_id).or_default().push(tag);
    }

    Ok(map)
}

fn into_summary(row: news::Model, tags: Vec<String>) -> NewsSummary {
    NewsSummary {
        id: row.id,
        title: row.title,
        publish_time: row.publish_time,
        source_url: row.source_url,
        cover: row.cover,
        news_type: row.news_type.to_value(),
        tags,
        video_url: row.video_url,
        intro: row.intro,
    }
}
