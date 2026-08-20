use std::collections::HashMap;

use akasha_application::news::{NewsCount, NewsSeries, NewsTag};
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
    RelationTrait,
    sea_query::{Expr, Func},
};

use crate::{
    Db, DbError,
    entities::{games, news, news_tags, news_tags_link},
    repositories::news::{UNTAGGED_TAG_FILTER, recent_by_tags, recent_untagged},
};

/// 查找包含至少一个视频的标签剧集及其首个视频信息
pub async fn find_series(
    db: &Db,
    game_id: &str,
    source_id: &str,
    tag_name: &str,
) -> Result<Option<NewsSeries>, DbError> {
    let tag_id = (
        tag_name.to_owned(),
        game_id.to_owned(),
        source_id.to_owned(),
    );
    if news_tags::Entity::find_by_id(tag_id)
        .one(db.conn())
        .await
        .map_err(DbError::Query)?
        .is_none()
    {
        return Ok(None);
    }

    // 仅在数据库中统计并读取该标签最早的一条视频
    let tagged_news_ids = news_tags_link::Entity::find()
        .select_only()
        .column(news_tags_link::Column::NewsId)
        .filter(news_tags_link::Column::GameId.eq(game_id))
        .filter(news_tags_link::Column::SourceId.eq(source_id))
        .filter(news_tags_link::Column::Name.eq(tag_name))
        .into_query();
    let videos = news::Entity::find()
        .filter(news::Column::GameId.eq(game_id))
        .filter(news::Column::SourceId.eq(source_id))
        .filter(news::Column::NewsType.eq(news::NewsType::Video))
        .filter(news::Column::Id.in_subquery(tagged_news_ids));
    let episode_count = videos
        .clone()
        .count(db.conn())
        .await
        .map_err(DbError::Query)?;
    let Some(first_video) = videos
        .order_by_asc(news::Column::PublishTime)
        .order_by_asc(news::Column::Id)
        .one(db.conn())
        .await
        .map_err(DbError::Query)?
    else {
        return Ok(None);
    };

    // 游戏名称用于生成稳定且容易识别的剧集标题
    let Some((game_name, game_cover)) = games::Entity::find_by_id(game_id)
        .select_only()
        .column(games::Column::NameZh)
        .column(games::Column::Cover)
        .into_tuple::<(String, Option<String>)>()
        .one(db.conn())
        .await
        .map_err(DbError::Query)?
    else {
        return Ok(None);
    };

    Ok(Some(NewsSeries {
        tag_name: tag_name.to_owned(),
        game_name,
        cover: first_video.cover.or(game_cover),
        premiered: first_video.publish_time,
        episode_count,
    }))
}

/// 列出来源标签及其统计和最近新闻预览
pub async fn list_tags(db: &Db, game_id: &str, source_id: &str) -> Result<Vec<NewsTag>, DbError> {
    let rows = news_tags::Entity::find()
        .filter(news_tags::Column::GameId.eq(game_id))
        .filter(news_tags::Column::SourceId.eq(source_id))
        .order_by_asc(news_tags::Column::GroupIndex)
        .order_by_asc(news_tags::Column::Group)
        .order_by(news_tags::Column::Index, sea_orm::Order::Asc)
        .order_by_asc(news_tags::Column::Name)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    let stats_by_name = tag_news_stats(db, game_id, source_id).await?;
    let mut recent_by_name = recent_by_tags(db, game_id, source_id).await?;

    let mut tags = Vec::with_capacity(rows.len() + 1);
    for row in rows {
        let recent = recent_by_name.remove(&row.name).unwrap_or_default();

        tags.push(NewsTag {
            news_count: stats_by_name.get(&row.name).copied().unwrap_or_default(),
            name: row.name,
            untagged: false,
            index: row.index,
            group: row.group,
            group_index: row.group_index,
            recent,
        });
    }

    tags.push(NewsTag {
        name: UNTAGGED_TAG_FILTER.to_owned(),
        untagged: true,
        index: 0,
        group: Some(UNTAGGED_TAG_FILTER.to_owned()),
        group_index: None,
        news_count: untagged_news_count(db, game_id, source_id).await?,
        recent: recent_untagged(db, game_id, source_id).await?,
    });

    Ok(tags)
}

/// 统计一个游戏来源下未关联标签的新闻
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

/// 聚合一个游戏来源下每个标签的新闻数量
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
