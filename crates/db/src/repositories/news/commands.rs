use sea_orm::{
    ActiveEnum, ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbErr, EntityTrait, QueryFilter,
    TransactionError, TransactionTrait, prelude::DateTimeWithTimeZone,
};

use crate::{
    Db, DbError,
    entities::{news, news_tags_link},
};

use super::projections::{NewsSummary, UpdateNewsResult};

/// 更新或创建新闻，并同步标签和原始数据
pub async fn update_news(db: &Db, input: UpdateNewsInput) -> Result<UpdateNewsResult, DbError> {
    db.conn()
        .transaction::<_, UpdateNewsResult, DbErr>(|txn| {
            Box::pin(async move {
                let news_type = news::NewsType::try_from_value(&input.news_type)?;
                let existing = news::Entity::find_by_id((
                    input.game_id.clone(),
                    input.source_id.clone(),
                    input.id.clone(),
                ))
                .one(txn)
                .await?;

                let created = existing.is_none();

                if let Some(row) = existing {
                    let mut active: news::ActiveModel = row.into();
                    active.title = Set(input.title.clone());
                    active.intro = Set(input.intro.clone());
                    active.publish_time = Set(input.publish_time);
                    active.source_url = Set(input.source_url.clone());
                    active.cover = Set(input.cover.clone());
                    active.news_type = Set(news_type);
                    active.video_url = Set(input.video_url.clone());
                    active.raw_data = Set(input.raw_data.clone());
                    active.update(txn).await?;
                } else {
                    news::ActiveModel {
                        game_id: Set(input.game_id.clone()),
                        source_id: Set(input.source_id.clone()),
                        id: Set(input.id.clone()),
                        title: Set(input.title.clone()),
                        intro: Set(input.intro.clone()),
                        publish_time: Set(input.publish_time),
                        source_url: Set(input.source_url.clone()),
                        cover: Set(input.cover.clone()),
                        news_type: Set(news_type),
                        video_url: Set(input.video_url.clone()),
                        raw_data: Set(input.raw_data.clone()),
                    }
                    .insert(txn)
                    .await?;
                }

                news_tags_link::Entity::delete_many()
                    .filter(news_tags_link::Column::GameId.eq(&input.game_id))
                    .filter(news_tags_link::Column::SourceId.eq(&input.source_id))
                    .filter(news_tags_link::Column::NewsId.eq(&input.id))
                    .exec(txn)
                    .await?;

                for tag in &input.tags {
                    news_tags_link::ActiveModel {
                        game_id: Set(input.game_id.clone()),
                        source_id: Set(input.source_id.clone()),
                        news_id: Set(input.id.clone()),
                        name: Set(tag.clone()),
                    }
                    .insert(txn)
                    .await?;
                }

                Ok(UpdateNewsResult {
                    created,
                    news: NewsSummary {
                        id: input.id,
                        title: input.title,
                        publish_time: input.publish_time,
                        source_url: input.source_url,
                        cover: input.cover,
                        news_type: input.news_type,
                        tags: input.tags,
                        video_url: input.video_url,
                        intro: input.intro,
                    },
                })
            })
        })
        .await
        .map_err(|error| match error {
            TransactionError::Connection(error) | TransactionError::Transaction(error) => {
                DbError::Query(error)
            }
        })
}

/// 批量替换已有新闻的标签关联
pub async fn update_tags(db: &Db, input: UpdateNewsTagsInput) -> Result<(), DbError> {
    db.conn()
        .transaction::<_, (), DbErr>(|txn| {
            Box::pin(async move {
                for update in input.updates {
                    news_tags_link::Entity::delete_many()
                        .filter(news_tags_link::Column::GameId.eq(&input.game_id))
                        .filter(news_tags_link::Column::SourceId.eq(&input.source_id))
                        .filter(news_tags_link::Column::NewsId.eq(&update.id))
                        .exec(txn)
                        .await?;

                    for tag in update.tags {
                        news_tags_link::ActiveModel {
                            game_id: Set(input.game_id.clone()),
                            source_id: Set(input.source_id.clone()),
                            news_id: Set(update.id.clone()),
                            name: Set(tag),
                        }
                        .insert(txn)
                        .await?;
                    }
                }

                Ok(())
            })
        })
        .await
        .map_err(|error| match error {
            TransactionError::Connection(error) | TransactionError::Transaction(error) => {
                DbError::Query(error)
            }
        })
}

pub struct UpdateNewsInput {
    pub game_id: String,
    pub source_id: String,
    pub id: String,
    pub title: String,
    pub intro: Option<String>,
    pub publish_time: DateTimeWithTimeZone,
    pub source_url: String,
    pub cover: Option<String>,
    pub news_type: String,
    pub video_url: Option<String>,
    pub tags: Vec<String>,
    pub raw_data: serde_json::Value,
}

/// 单条新闻的标签替换请求
pub struct UpdateNewsTagsItem {
    pub id: String,
    pub tags: Vec<String>,
}

/// 同一来源的一批标签替换请求
pub struct UpdateNewsTagsInput {
    pub game_id: String,
    pub source_id: String,
    pub updates: Vec<UpdateNewsTagsItem>,
}
