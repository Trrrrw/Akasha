use std::collections::HashMap;

use akasha_application::news::{
    ListNewsFilter, ListNewsRawFilter, NewsCharacter, NewsFeedFilter, NewsFilter, NewsOrder,
    NewsRawItem, NewsSource, NewsSummary, RecentNews, VideoPlayback,
};
use chrono::Utc;
use sea_orm::{
    ActiveEnum, ColumnTrait, Condition, EntityTrait, JoinType, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, RelationTrait, Select, sea_query::Expr,
};

use crate::{
    Db, DbError,
    entities::{
        news, news_sources, news_tags_link, sr_game_data, sr_news_characters_link, ys_game_data,
        ys_news_characters_link, zzz_game_data, zzz_news_characters_link,
    },
    models::text_query_condition,
};

/// 标签列表内部用于表示未分类统计的保留名称
pub const UNTAGGED_TAG_FILTER: &str = "__untagged__";

/// 列出一个游戏已配置的全部新闻来源
pub async fn list_sources(db: &Db, game_id: &str) -> Result<Vec<NewsSource>, DbError> {
    let rows = news_sources::Entity::find()
        .filter(news_sources::Column::GameId.eq(game_id))
        .order_by(news_sources::Column::Index, sea_orm::Order::Asc)
        .order_by_asc(news_sources::Column::Id)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    Ok(rows.into_iter().map(NewsSource::from).collect())
}

/// 列出经过筛选和分页的新闻集合
pub async fn list(db: &Db, filter: ListNewsFilter) -> Result<(u64, Vec<NewsSummary>), DbError> {
    let query = filtered_query(&filter.filter)?;
    let total = query
        .clone()
        .count(db.conn())
        .await
        .map_err(DbError::Query)?;

    let rows = ordered_query(query, filter.order)
        .limit(filter.limit)
        .offset(filter.offset)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    let items =
        summaries_with_tags(db, &filter.filter.game_id, &filter.filter.source_id, rows).await?;

    Ok((total, items))
}

/// 按列表共用筛选条件读取固定发布时间倒序的 RSS 新闻
pub async fn list_feed(db: &Db, filter: NewsFeedFilter) -> Result<Vec<NewsSummary>, DbError> {
    let rows = ordered_query(filtered_query(&filter.filter)?, NewsOrder::Desc)
        .limit(filter.limit)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    summaries_with_tags(db, &filter.filter.game_id, &filter.filter.source_id, rows).await
}

/// 构造新闻列表与 RSS 完全共用的数据库筛选条件
fn filtered_query(filter: &NewsFilter) -> Result<Select<news::Entity>, DbError> {
    let mut query = news::Entity::find()
        .filter(news::Column::GameId.eq(&filter.game_id))
        .filter(news::Column::SourceId.eq(&filter.source_id));

    if let Some(start) = filter.start_publish_time {
        let start = start.with_timezone(&Utc).fixed_offset();
        query = query.filter(news::Column::PublishTime.gte(start));
    }

    if let Some(end) = filter.end_publish_time {
        let end = end.with_timezone(&Utc).fixed_offset();
        query = query.filter(news::Column::PublishTime.lt(end));
    }

    if let Some(news_type) = filter.news_type.as_ref() {
        let news_type = news::NewsType::try_from_value(news_type).map_err(DbError::Query)?;
        query = query.filter(news::Column::NewsType.eq(news_type));
    }

    if !filter.tags.is_empty() || filter.include_untagged {
        let mut tag_conditions = Condition::any();

        if !filter.tags.is_empty() {
            let tag_news_ids = news_tags_link::Entity::find()
                .select_only()
                .column(news_tags_link::Column::NewsId)
                .filter(news_tags_link::Column::GameId.eq(&filter.game_id))
                .filter(news_tags_link::Column::SourceId.eq(&filter.source_id))
                .filter(news_tags_link::Column::Name.is_in(filter.tags.iter().cloned()))
                .into_query();

            tag_conditions = tag_conditions.add(news::Column::Id.in_subquery(tag_news_ids));
        }

        if filter.include_untagged {
            let tagged_news_ids = news_tags_link::Entity::find()
                .select_only()
                .column(news_tags_link::Column::NewsId)
                .filter(news_tags_link::Column::GameId.eq(&filter.game_id))
                .filter(news_tags_link::Column::SourceId.eq(&filter.source_id))
                .into_query();

            tag_conditions = tag_conditions.add(news::Column::Id.not_in_subquery(tagged_news_ids));
        }

        query = query.filter(tag_conditions);
    }

    if !filter.character_ids.is_empty() {
        let character_news_ids = match filter.game_id.as_str() {
            "ys" => ys_news_characters_link::Entity::find()
                .select_only()
                .column(ys_news_characters_link::Column::NewsId)
                .filter(ys_news_characters_link::Column::GameId.eq(&filter.game_id))
                .filter(ys_news_characters_link::Column::SourceId.eq(&filter.source_id))
                .filter(
                    ys_news_characters_link::Column::CharacterId
                        .is_in(filter.character_ids.iter().cloned()),
                )
                .into_query(),
            "sr" => sr_news_characters_link::Entity::find()
                .select_only()
                .column(sr_news_characters_link::Column::NewsId)
                .filter(sr_news_characters_link::Column::GameId.eq(&filter.game_id))
                .filter(sr_news_characters_link::Column::SourceId.eq(&filter.source_id))
                .filter(
                    sr_news_characters_link::Column::CharacterId
                        .is_in(filter.character_ids.iter().cloned()),
                )
                .into_query(),
            "zzz" => zzz_news_characters_link::Entity::find()
                .select_only()
                .column(zzz_news_characters_link::Column::NewsId)
                .filter(zzz_news_characters_link::Column::GameId.eq(&filter.game_id))
                .filter(zzz_news_characters_link::Column::SourceId.eq(&filter.source_id))
                .filter(
                    zzz_news_characters_link::Column::CharacterId
                        .is_in(filter.character_ids.iter().cloned()),
                )
                .into_query(),
            game_id => {
                return Err(DbError::Query(sea_orm::DbErr::Custom(format!(
                    "character filtering is not supported for game {game_id}"
                ))));
            }
        };
        query = query.filter(news::Column::Id.in_subquery(character_news_ids));
    }

    if let Some(title_query) = filter.title_query.as_ref() {
        query = query.filter(text_query_condition(
            title_query,
            &[Expr::col(news::Column::Title)],
        ));
    }

    Ok(query)
}

/// 为新闻查询应用稳定的发布时间与新闻 ID 排序
fn ordered_query(query: Select<news::Entity>, order: NewsOrder) -> Select<news::Entity> {
    let order = match order {
        NewsOrder::Asc => sea_orm::Order::Asc,
        NewsOrder::Desc => sea_orm::Order::Desc,
    };

    query
        .order_by(news::Column::PublishTime, order.clone())
        .order_by(news::Column::Id, order)
}

/// 按稳定的新闻 ID 顺序读取维护任务需要的原始新闻
pub async fn list_raw(
    db: &Db,
    filter: ListNewsRawFilter,
) -> Result<(u64, Vec<NewsRawItem>), DbError> {
    let mut query = news::Entity::find()
        .filter(news::Column::GameId.eq(&filter.game_id))
        .filter(news::Column::SourceId.eq(&filter.source_id));

    if let Some(news_id) = filter.news_id.as_deref() {
        query = query.filter(news::Column::Id.eq(news_id));
    } else if let Some(after_id) = filter.after_id.as_deref() {
        query = query.filter(news::Column::Id.gt(after_id));
    }

    if let Some(news_type) = filter.news_type {
        let news_type = news::NewsType::try_from_value(&news_type).map_err(DbError::Query)?;
        query = query.filter(news::Column::NewsType.eq(news_type));
    }

    if !filter.tags.is_empty() {
        let tagged_news_ids = news_tags_link::Entity::find()
            .select_only()
            .column(news_tags_link::Column::NewsId)
            .filter(news_tags_link::Column::GameId.eq(&filter.game_id))
            .filter(news_tags_link::Column::SourceId.eq(&filter.source_id))
            .filter(news_tags_link::Column::Name.is_in(filter.tags))
            .into_query();
        query = query.filter(news::Column::Id.in_subquery(tagged_news_ids));
    }

    let total = query
        .clone()
        .count(db.conn())
        .await
        .map_err(DbError::Query)?;
    let rows = query
        .order_by_asc(news::Column::Id)
        .limit(filter.limit)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    let news_ids = rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
    let mut tags_map = news_tags_map(db, &filter.game_id, &filter.source_id, &news_ids).await?;

    let items = rows
        .into_iter()
        .map(|row| {
            let tags = tags_map.remove(&row.id).unwrap_or_default();
            into_raw_item(row, tags)
        })
        .collect();

    Ok((total, items))
}

/// 查找一条新闻并解析其全部标签
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
            let mut characters_map =
                news_characters_map(db, game_id, source_id, &[news_id.to_owned()]).await?;
            let characters = characters_map.remove(news_id).unwrap_or_default();

            Ok(Some(into_summary(row, tags, characters)))
        }
        None => Ok(None),
    }
}

/// 按共同标签数量和发布时间列出同一来源的相关视频
pub async fn list_related_videos(
    db: &Db,
    game_id: &str,
    source_id: &str,
    news_id: &str,
    tags: &[String],
    limit: u64,
) -> Result<Vec<NewsSummary>, DbError> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    // 先在数据库中聚合共同标签，并只保留最终需要的候选 ID
    let ranked_ids = if tags.is_empty() {
        Vec::new()
    } else {
        news_tags_link::Entity::find()
            .select_only()
            .column(news_tags_link::Column::NewsId)
            .join(JoinType::InnerJoin, news_tags_link::Relation::News.def())
            .filter(news_tags_link::Column::GameId.eq(game_id))
            .filter(news_tags_link::Column::SourceId.eq(source_id))
            .filter(news_tags_link::Column::NewsId.ne(news_id))
            .filter(news_tags_link::Column::Name.is_in(tags.iter().cloned()))
            .filter(news::Column::NewsType.eq(news::NewsType::Video))
            .group_by(news_tags_link::Column::NewsId)
            .group_by(news::Column::PublishTime)
            .order_by_desc(news_tags_link::Column::Name.count())
            .order_by_desc(news::Column::PublishTime)
            .order_by_desc(news_tags_link::Column::NewsId)
            .limit(limit)
            .into_tuple::<String>()
            .all(db.conn())
            .await
            .map_err(DbError::Query)?
    };

    // 没有标签或没有匹配项时，回退到同一来源的最新视频
    let rows = if ranked_ids.is_empty() {
        news::Entity::find()
            .filter(news::Column::GameId.eq(game_id))
            .filter(news::Column::SourceId.eq(source_id))
            .filter(news::Column::Id.ne(news_id))
            .filter(news::Column::NewsType.eq(news::NewsType::Video))
            .order_by_desc(news::Column::PublishTime)
            .order_by_desc(news::Column::Id)
            .limit(limit)
            .all(db.conn())
            .await
            .map_err(DbError::Query)?
    } else {
        let rows = news::Entity::find()
            .filter(news::Column::GameId.eq(game_id))
            .filter(news::Column::SourceId.eq(source_id))
            .filter(news::Column::Id.is_in(ranked_ids.iter().cloned()))
            .all(db.conn())
            .await
            .map_err(DbError::Query)?;
        let mut rows_by_id = rows
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect::<HashMap<_, _>>();

        ranked_ids
            .into_iter()
            .filter_map(|id| rows_by_id.remove(&id))
            .collect()
    };

    summaries_with_tags(db, game_id, source_id, rows).await
}

/// 批量加载全部标签各自最新的文章和视频
pub async fn recent_by_tags(
    db: &Db,
    game_id: &str,
    source_id: &str,
) -> Result<HashMap<String, RecentNews>, DbError> {
    let articles = latest_by_tags_and_type(db, game_id, source_id, news::NewsType::Article).await?;
    let videos = latest_by_tags_and_type(db, game_id, source_id, news::NewsType::Video).await?;
    let news_ids = articles
        .iter()
        .chain(videos.iter())
        .map(|(_, row)| row.id.clone())
        .collect::<Vec<_>>();
    let tags_map = if news_ids.is_empty() {
        HashMap::new()
    } else {
        news_tags_map(db, game_id, source_id, &news_ids).await?
    };
    let characters_map = if news_ids.is_empty() {
        HashMap::new()
    } else {
        news_characters_map(db, game_id, source_id, &news_ids).await?
    };
    let mut recent_by_name = HashMap::<String, RecentNews>::new();

    // 同一条新闻可能同时是多个标签的最近条目，因此标签集合需要按条目克隆
    for (tag_name, row) in articles {
        let tags = tags_map.get(&row.id).cloned().unwrap_or_default();
        let characters = characters_map.get(&row.id).cloned().unwrap_or_default();
        recent_by_name
            .entry(tag_name)
            .or_default()
            .article
            .push(into_summary(row, tags, characters));
    }
    for (tag_name, row) in videos {
        let tags = tags_map.get(&row.id).cloned().unwrap_or_default();
        let characters = characters_map.get(&row.id).cloned().unwrap_or_default();
        recent_by_name
            .entry(tag_name)
            .or_default()
            .video
            .push(into_summary(row, tags, characters));
    }

    Ok(recent_by_name)
}

/// 加载最新的未分类文章和视频
pub async fn recent_untagged(
    db: &Db,
    game_id: &str,
    source_id: &str,
) -> Result<RecentNews, DbError> {
    let article = latest_untagged_by_type(db, game_id, source_id, news::NewsType::Article).await?;
    let video = latest_untagged_by_type(db, game_id, source_id, news::NewsType::Video).await?;

    Ok(RecentNews {
        article: summaries_with_tags(db, game_id, source_id, article.into_iter().collect()).await?,
        video: summaries_with_tags(db, game_id, source_id, video.into_iter().collect()).await?,
    })
}

/// 加载一个游戏最新的文章和视频
pub async fn recent_by_game(db: &Db, game_id: &str) -> Result<RecentNews, DbError> {
    let article = latest_by_game_and_type(db, game_id, news::NewsType::Article).await?;
    let video = latest_by_game_and_type(db, game_id, news::NewsType::Video).await?;

    Ok(RecentNews {
        article: match article {
            Some(row) => vec![summary_with_tags(db, row).await?],
            None => Vec::new(),
        },
        video: match video {
            Some(row) => vec![summary_with_tags(db, row).await?],
            None => Vec::new(),
        },
    })
}

/// 按标签逐项查找指定类型最新新闻，避免依赖数据库专用去重语法
async fn latest_by_tags_and_type(
    db: &Db,
    game_id: &str,
    source_id: &str,
    news_type: news::NewsType,
) -> Result<Vec<(String, news::Model)>, DbError> {
    let tag_names = news_tags_link::Entity::find()
        .select_only()
        .column(news_tags_link::Column::Name)
        .join(JoinType::InnerJoin, news_tags_link::Relation::News.def())
        .filter(news_tags_link::Column::GameId.eq(game_id))
        .filter(news_tags_link::Column::SourceId.eq(source_id))
        .filter(news::Column::NewsType.eq(news_type.clone()))
        .distinct()
        .order_by_asc(news_tags_link::Column::Name)
        .into_tuple::<String>()
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    let mut rows = Vec::with_capacity(tag_names.len());
    for tag_name in tag_names {
        let row = news_tags_link::Entity::find()
            .find_also_related(news::Entity)
            .filter(news_tags_link::Column::GameId.eq(game_id))
            .filter(news_tags_link::Column::SourceId.eq(source_id))
            .filter(news_tags_link::Column::Name.eq(&tag_name))
            .filter(news::Column::NewsType.eq(news_type.clone()))
            .order_by_desc(news::Column::PublishTime)
            .order_by_desc(news::Column::Id)
            .limit(1)
            .one(db.conn())
            .await
            .map_err(DbError::Query)?;

        if let Some((_, Some(news))) = row {
            rows.push((tag_name, news));
        }
    }

    Ok(rows)
}

/// 查找指定类型最新的未分类新闻
async fn latest_untagged_by_type(
    db: &Db,
    game_id: &str,
    source_id: &str,
    news_type: news::NewsType,
) -> Result<Option<news::Model>, DbError> {
    let tagged_news_ids = news_tags_link::Entity::find()
        .select_only()
        .column(news_tags_link::Column::NewsId)
        .filter(news_tags_link::Column::GameId.eq(game_id))
        .filter(news_tags_link::Column::SourceId.eq(source_id))
        .into_query();

    news::Entity::find()
        .filter(news::Column::GameId.eq(game_id))
        .filter(news::Column::SourceId.eq(source_id))
        .filter(news::Column::NewsType.eq(news_type))
        .filter(news::Column::Id.not_in_subquery(tagged_news_ids))
        .order_by_desc(news::Column::PublishTime)
        .order_by_desc(news::Column::Id)
        .limit(1)
        .one(db.conn())
        .await
        .map_err(DbError::Query)
}

/// 查找一个游戏指定类型的最新新闻
async fn latest_by_game_and_type(
    db: &Db,
    game_id: &str,
    news_type: news::NewsType,
) -> Result<Option<news::Model>, DbError> {
    news::Entity::find()
        .filter(news::Column::GameId.eq(game_id))
        .filter(news::Column::NewsType.eq(news_type))
        .order_by_desc(news::Column::PublishTime)
        .order_by_desc(news::Column::Id)
        .limit(1)
        .one(db.conn())
        .await
        .map_err(DbError::Query)
}

/// 为一条数据库新闻记录补充标签名称
async fn summary_with_tags(db: &Db, row: news::Model) -> Result<NewsSummary, DbError> {
    let tags = news_tags_link::Entity::find()
        .select_only()
        .column(news_tags_link::Column::Name)
        .filter(news_tags_link::Column::GameId.eq(&row.game_id))
        .filter(news_tags_link::Column::SourceId.eq(&row.source_id))
        .filter(news_tags_link::Column::NewsId.eq(&row.id))
        .into_tuple::<String>()
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;
    let mut characters_map = news_characters_map(
        db,
        &row.game_id,
        &row.source_id,
        std::slice::from_ref(&row.id),
    )
    .await?;
    let characters = characters_map.remove(&row.id).unwrap_or_default();

    Ok(into_summary(row, tags, characters))
}

/// 为一组已排序的数据库新闻记录批量补充标签
async fn summaries_with_tags(
    db: &Db,
    game_id: &str,
    source_id: &str,
    rows: Vec<news::Model>,
) -> Result<Vec<NewsSummary>, DbError> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let news_ids = rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
    let mut tags_map = news_tags_map(db, game_id, source_id, &news_ids).await?;
    let mut characters_map = news_characters_map(db, game_id, source_id, &news_ids).await?;
    let summaries = rows
        .into_iter()
        .map(|row| {
            let tags = tags_map.remove(&row.id).unwrap_or_default();
            let characters = characters_map.remove(&row.id).unwrap_or_default();
            into_summary(row, tags, characters)
        })
        .collect();

    Ok(summaries)
}

/// 通过一次查询获取指定新闻分页的角色
async fn news_characters_map(
    db: &Db,
    game_id: &str,
    source_id: &str,
    news_ids: &[String],
) -> Result<HashMap<String, Vec<NewsCharacter>>, DbError> {
    if news_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = match game_id {
        "ys" => {
            ys_news_characters_link::Entity::find()
                .select_only()
                .column(ys_news_characters_link::Column::NewsId)
                .column(ys_news_characters_link::Column::CharacterId)
                .column(ys_game_data::Column::Name)
                .join(
                    JoinType::InnerJoin,
                    ys_news_characters_link::Relation::YsGameData.def(),
                )
                .filter(ys_news_characters_link::Column::GameId.eq(game_id))
                .filter(ys_news_characters_link::Column::SourceId.eq(source_id))
                .filter(ys_news_characters_link::Column::NewsId.is_in(news_ids.iter().cloned()))
                .order_by_asc(ys_news_characters_link::Column::CharacterId)
                .into_tuple::<(String, String, String)>()
                .all(db.conn())
                .await
        }
        "sr" => {
            sr_news_characters_link::Entity::find()
                .select_only()
                .column(sr_news_characters_link::Column::NewsId)
                .column(sr_news_characters_link::Column::CharacterId)
                .column(sr_game_data::Column::Name)
                .join(
                    JoinType::InnerJoin,
                    sr_news_characters_link::Relation::SrGameData.def(),
                )
                .filter(sr_news_characters_link::Column::GameId.eq(game_id))
                .filter(sr_news_characters_link::Column::SourceId.eq(source_id))
                .filter(sr_news_characters_link::Column::NewsId.is_in(news_ids.iter().cloned()))
                .order_by_asc(sr_news_characters_link::Column::CharacterId)
                .into_tuple::<(String, String, String)>()
                .all(db.conn())
                .await
        }
        "zzz" => {
            zzz_news_characters_link::Entity::find()
                .select_only()
                .column(zzz_news_characters_link::Column::NewsId)
                .column(zzz_news_characters_link::Column::CharacterId)
                .column(zzz_game_data::Column::Name)
                .join(
                    JoinType::InnerJoin,
                    zzz_news_characters_link::Relation::ZzzGameData.def(),
                )
                .filter(zzz_news_characters_link::Column::GameId.eq(game_id))
                .filter(zzz_news_characters_link::Column::SourceId.eq(source_id))
                .filter(zzz_news_characters_link::Column::NewsId.is_in(news_ids.iter().cloned()))
                .order_by_asc(zzz_news_characters_link::Column::CharacterId)
                .into_tuple::<(String, String, String)>()
                .all(db.conn())
                .await
        }
        _ => return Ok(HashMap::new()),
    }
    .map_err(DbError::Query)?;

    let mut map: HashMap<String, Vec<NewsCharacter>> = HashMap::new();
    for (news_id, id, name) in rows {
        map.entry(news_id)
            .or_default()
            .push(NewsCharacter { id, name });
    }

    Ok(map)
}

/// 通过一次查询获取指定新闻分页的标签
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

/// 将数据库记录和预加载标签映射为应用层读取模型
fn into_summary(
    row: news::Model,
    tags: Vec<String>,
    characters: Vec<NewsCharacter>,
) -> NewsSummary {
    NewsSummary {
        id: row.id,
        source_id: row.source_id,
        title: row.title,
        publish_time: row.publish_time,
        source_url: row.source_url,
        cover: row.cover,
        news_type: row.news_type.to_value(),
        tags,
        characters,
        video_url: row.video_url,
        video_playback: row.video_playback.map(app_video_playback),
        video_duration_ms: row.video_duration_ms,
        intro: row.intro,
    }
}

/// 将数据库记录和标签转换为维护任务模型
fn into_raw_item(row: news::Model, tags: Vec<String>) -> NewsRawItem {
    NewsRawItem {
        id: row.id,
        title: row.title,
        intro: row.intro,
        publish_time: row.publish_time,
        source_url: row.source_url,
        cover: row.cover,
        news_type: row.news_type.to_value(),
        tags,
        video_url: row.video_url,
        video_playback: row.video_playback.map(app_video_playback),
        video_duration_ms: row.video_duration_ms,
        raw_data: row.raw_data,
    }
}

/// 将数据库播放方式转换为应用层枚举
const fn app_video_playback(value: news::VideoPlayback) -> VideoPlayback {
    match value {
        news::VideoPlayback::Direct => VideoPlayback::Direct,
        news::VideoPlayback::Embed => VideoPlayback::Embed,
    }
}
