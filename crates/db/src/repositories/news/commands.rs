use akasha_application::news::{
    NewsSummary, ReplaceNewsTagsCommand, UpdateNewsCommand, UpdateNewsResult,
};
use sea_orm::{
    ActiveEnum, ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbErr, EntityTrait, QueryFilter,
    QuerySelect, TransactionError, TransactionTrait,
};
use serde_json::json;

use crate::{
    Db, DbError,
    entities::{news, news_tags_link},
    repositories::audit,
};

/// 创建或更新新闻，同时替换其标签和原始来源数据
pub async fn update_news(db: &Db, command: UpdateNewsCommand) -> Result<UpdateNewsResult, DbError> {
    db.conn()
        .transaction::<_, UpdateNewsResult, DbErr>(|txn| {
            Box::pin(async move {
                // 先解析新闻类型，并根据是否已有记录执行更新或插入
                let news_type = news::NewsType::try_from_value(&command.news_type)?;
                let existing = news::Entity::find_by_id((
                    command.game_id.clone(),
                    command.source_id.clone(),
                    command.id.clone(),
                ))
                .one(txn)
                .await?;

                let created = existing.is_none();
                let existing_tags = if existing.is_some() {
                    news_tags_link::Entity::find()
                        .select_only()
                        .column(news_tags_link::Column::Name)
                        .filter(news_tags_link::Column::GameId.eq(&command.game_id))
                        .filter(news_tags_link::Column::SourceId.eq(&command.source_id))
                        .filter(news_tags_link::Column::NewsId.eq(&command.id))
                        .into_tuple::<String>()
                        .all(txn)
                        .await?
                } else {
                    Vec::new()
                };
                let changed_fields = existing
                    .as_ref()
                    .map(|row| changed_news_fields(row, &command, &existing_tags))
                    .unwrap_or_else(created_news_fields);

                if let Some(row) = existing {
                    let mut active: news::ActiveModel = row.into();
                    active.title = Set(command.title.clone());
                    active.intro = Set(command.intro.clone());
                    active.publish_time = Set(command.publish_time);
                    active.source_url = Set(command.source_url.clone());
                    active.cover = Set(command.cover.clone());
                    active.news_type = Set(news_type);
                    active.video_url = Set(command.video_url.clone());
                    active.video_duration_ms = Set(command.video_duration_ms);
                    active.raw_data = Set(command.raw_data.clone());
                    active.update(txn).await?;
                } else {
                    news::ActiveModel {
                        game_id: Set(command.game_id.clone()),
                        source_id: Set(command.source_id.clone()),
                        id: Set(command.id.clone()),
                        title: Set(command.title.clone()),
                        intro: Set(command.intro.clone()),
                        publish_time: Set(command.publish_time),
                        source_url: Set(command.source_url.clone()),
                        cover: Set(command.cover.clone()),
                        news_type: Set(news_type),
                        video_url: Set(command.video_url.clone()),
                        video_duration_ms: Set(command.video_duration_ms),
                        raw_data: Set(command.raw_data.clone()),
                    }
                    .insert(txn)
                    .await?;
                }

                // 删除旧标签关联后写入请求提供的完整标签集合
                news_tags_link::Entity::delete_many()
                    .filter(news_tags_link::Column::GameId.eq(&command.game_id))
                    .filter(news_tags_link::Column::SourceId.eq(&command.source_id))
                    .filter(news_tags_link::Column::NewsId.eq(&command.id))
                    .exec(txn)
                    .await?;

                for tag in &command.tags {
                    news_tags_link::ActiveModel {
                        game_id: Set(command.game_id.clone()),
                        source_id: Set(command.source_id.clone()),
                        news_id: Set(command.id.clone()),
                        name: Set(tag.clone()),
                    }
                    .insert(txn)
                    .await?;
                }

                if created || !changed_fields.is_empty() {
                    audit::insert(
                        txn,
                        &command.audit,
                        "news.upsert",
                        Some("news"),
                        Some(format!(
                            "{}:{}:{}",
                            command.game_id, command.source_id, command.id
                        )),
                        json!({
                            "created": created,
                            "changed_fields": changed_fields,
                        }),
                    )
                    .await?;
                }

                // 使用命令数据构造应用层结果，避免再次读取刚写入的记录
                Ok(UpdateNewsResult {
                    created,
                    item: NewsSummary {
                        id: command.id,
                        source_id: command.source_id,
                        title: command.title,
                        publish_time: command.publish_time,
                        source_url: command.source_url,
                        cover: command.cover,
                        news_type: command.news_type,
                        tags: command.tags,
                        video_url: command.video_url,
                        video_duration_ms: command.video_duration_ms,
                        intro: command.intro,
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

/// 计算一条已有新闻实际发生变化的字段
fn changed_news_fields(
    row: &news::Model,
    command: &UpdateNewsCommand,
    existing_tags: &[String],
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if row.title != command.title {
        fields.push("title");
    }
    if row.intro != command.intro {
        fields.push("intro");
    }
    if row.publish_time != command.publish_time {
        fields.push("publish_time");
    }
    if row.source_url != command.source_url {
        fields.push("source_url");
    }
    if row.cover != command.cover {
        fields.push("cover");
    }
    if row.news_type.to_value() != command.news_type {
        fields.push("news_type");
    }
    if row.video_url != command.video_url {
        fields.push("video_url");
    }
    if row.video_duration_ms != command.video_duration_ms {
        fields.push("video_duration_ms");
    }
    let mut previous_tags = existing_tags.to_vec();
    previous_tags.sort();
    let mut next_tags = command.tags.clone();
    next_tags.sort();
    if previous_tags != next_tags {
        fields.push("tags");
    }
    if row.raw_data != command.raw_data {
        fields.push("raw_data");
    }
    fields
}

/// 返回新建新闻会写入的字段集合
fn created_news_fields() -> Vec<&'static str> {
    let mut fields = vec![
        "title",
        "intro",
        "publish_time",
        "source_url",
        "cover",
        "news_type",
        "video_url",
        "video_duration_ms",
        "raw_data",
    ];
    fields.push("tags");
    fields
}

/// 替换一个来源下多条新闻的标签关联
pub async fn replace_news_tags(db: &Db, command: ReplaceNewsTagsCommand) -> Result<(), DbError> {
    db.conn()
        .transaction::<_, (), DbErr>(|txn| {
            Box::pin(async move {
                let update_count = command.updates.len();
                // 每条新闻独立替换全部标签，整体仍由同一个事务保证原子性
                for update in command.updates {
                    news_tags_link::Entity::delete_many()
                        .filter(news_tags_link::Column::GameId.eq(&command.game_id))
                        .filter(news_tags_link::Column::SourceId.eq(&command.source_id))
                        .filter(news_tags_link::Column::NewsId.eq(&update.id))
                        .exec(txn)
                        .await?;

                    for tag in update.tags {
                        news_tags_link::ActiveModel {
                            game_id: Set(command.game_id.clone()),
                            source_id: Set(command.source_id.clone()),
                            news_id: Set(update.id.clone()),
                            name: Set(tag),
                        }
                        .insert(txn)
                        .await?;
                    }
                }

                if update_count > 0 {
                    audit::insert(
                        txn,
                        &command.audit,
                        "news.tags_replace",
                        Some("news_source"),
                        Some(format!("{}:{}", command.game_id, command.source_id)),
                        json!({
                            "changed_fields": ["tags"],
                            "news_count": update_count,
                        }),
                    )
                    .await?;
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
