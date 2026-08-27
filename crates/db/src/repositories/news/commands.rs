use std::collections::HashSet;

use akasha_application::news::{
    NewsCharacter, NewsCharacterInput, NewsSummary, ReplaceNewsCharactersCommand,
    ReplaceNewsTagsCommand, UpdateNewsCommand, UpdateNewsResult,
};
use chrono::Utc;
use sea_orm::{
    ActiveEnum, ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseTransaction, DbErr,
    EntityTrait, QueryFilter, QuerySelect, TransactionError, TransactionTrait,
};
use serde_json::json;

use crate::{
    Db, DbError,
    entities::{
        news, news_tags_link, sr_news_characters_link, ys_news_characters_link,
        zzz_news_characters_link,
    },
    repositories::audit,
};

/// 创建或更新新闻，同时替换其标签、角色关联和原始来源数据
pub async fn update_news(db: &Db, command: UpdateNewsCommand) -> Result<UpdateNewsResult, DbError> {
    db.conn()
        .transaction::<_, UpdateNewsResult, DbErr>(|txn| {
            Box::pin(async move {
                let mut command = command;
                command.publish_time = command.publish_time.with_timezone(&Utc).fixed_offset();

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
                let existing_characters = if command.characters.is_some() && existing.is_some() {
                    list_news_character_ids(txn, &command.game_id, &command.source_id, &command.id)
                        .await?
                } else {
                    Vec::new()
                };
                if let Some(characters) = command.characters.as_ref() {
                    validate_character_inputs(characters)?;
                }
                let changed_fields = existing
                    .as_ref()
                    .map(|row| {
                        changed_news_fields(row, &command, &existing_tags, &existing_characters)
                    })
                    .unwrap_or_else(|| created_news_fields(command.characters.is_some()));

                if !created && changed_fields.is_empty() {
                    return Ok(UpdateNewsResult {
                        created: false,
                        changed: false,
                        item: news_summary(command),
                    });
                }

                if let Some(row) = existing {
                    if changed_fields
                        .iter()
                        .any(|field| *field != "tags" && *field != "characters")
                    {
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
                    }
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

                if created || changed_fields.contains(&"tags") {
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
                }

                if let Some(characters) = command
                    .characters
                    .as_ref()
                    .filter(|_| created || changed_fields.contains(&"characters"))
                {
                    replace_news_character_links(
                        txn,
                        &command.game_id,
                        &command.source_id,
                        &command.id,
                        characters,
                    )
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

                Ok(UpdateNewsResult {
                    created,
                    changed: true,
                    item: news_summary(command),
                })
            })
        })
        .await
        .map_err(transaction_error)
}

/// 将新闻写入命令转换为接口返回所需的公开摘要
fn news_summary(command: UpdateNewsCommand) -> NewsSummary {
    NewsSummary {
        id: command.id,
        source_id: command.source_id,
        title: command.title,
        publish_time: command.publish_time,
        source_url: command.source_url,
        cover: command.cover,
        news_type: command.news_type,
        tags: command.tags,
        characters: command
            .characters
            .unwrap_or_default()
            .into_iter()
            .map(NewsCharacter::from)
            .collect(),
        video_url: command.video_url,
        video_duration_ms: command.video_duration_ms,
        intro: command.intro,
    }
}

/// 计算一条已有新闻实际发生变化的字段
fn changed_news_fields(
    row: &news::Model,
    command: &UpdateNewsCommand,
    existing_tags: &[String],
    existing_characters: &[String],
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
    if let Some(characters) = command.characters.as_ref() {
        let mut previous_characters = existing_characters.to_vec();
        previous_characters.sort();
        let mut next_characters = characters
            .iter()
            .map(|character| character.id.clone())
            .collect::<Vec<_>>();
        next_characters.sort();
        if previous_characters != next_characters {
            fields.push("characters");
        }
    }
    if row.raw_data != command.raw_data {
        fields.push("raw_data");
    }
    fields
}

/// 返回新建新闻会写入的字段集合
fn created_news_fields(include_characters: bool) -> Vec<&'static str> {
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
        "tags",
    ];
    if include_characters {
        fields.push("characters");
    }
    fields
}

/// 校验一次完整写入中的角色 ID 不重复
fn validate_character_inputs(characters: &[NewsCharacterInput]) -> Result<(), DbErr> {
    let mut ids = HashSet::with_capacity(characters.len());
    for character in characters {
        if !ids.insert(character.id.clone()) {
            return Err(DbErr::Custom(format!(
                "duplicated news character id: {}",
                character.id
            )));
        }
    }
    Ok(())
}

/// 替换一个来源下多条新闻的标签关联
pub async fn replace_news_tags(db: &Db, command: ReplaceNewsTagsCommand) -> Result<(), DbError> {
    db.conn()
        .transaction::<_, (), DbErr>(|txn| {
            Box::pin(async move {
                let update_count = command.updates.len();
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
        .map_err(transaction_error)
}

/// 替换一个来源下多条新闻的游戏专属角色关联
pub async fn replace_news_characters(
    db: &Db,
    command: ReplaceNewsCharactersCommand,
) -> Result<(), DbError> {
    db.conn()
        .transaction::<_, (), DbErr>(|txn| {
            Box::pin(async move {
                let update_count = command.updates.len();
                for update in command.updates {
                    validate_character_inputs(&update.characters)?;
                    replace_news_character_links(
                        txn,
                        &command.game_id,
                        &command.source_id,
                        &update.id,
                        &update.characters,
                    )
                    .await?;
                }

                if update_count > 0 {
                    audit::insert(
                        txn,
                        &command.audit,
                        "news.characters_replace",
                        Some("news_source"),
                        Some(format!("{}:{}", command.game_id, command.source_id)),
                        json!({
                            "changed_fields": ["characters"],
                            "news_count": update_count,
                        }),
                    )
                    .await?;
                }

                Ok(())
            })
        })
        .await
        .map_err(transaction_error)
}

/// 读取一条新闻在对应游戏关联表中的角色 ID
async fn list_news_character_ids(
    txn: &DatabaseTransaction,
    game_id: &str,
    source_id: &str,
    news_id: &str,
) -> Result<Vec<String>, DbErr> {
    match game_id {
        "ys" => {
            ys_news_characters_link::Entity::find()
                .select_only()
                .column(ys_news_characters_link::Column::CharacterId)
                .filter(ys_news_characters_link::Column::GameId.eq(game_id))
                .filter(ys_news_characters_link::Column::SourceId.eq(source_id))
                .filter(ys_news_characters_link::Column::NewsId.eq(news_id))
                .into_tuple::<String>()
                .all(txn)
                .await
        }
        "sr" => {
            sr_news_characters_link::Entity::find()
                .select_only()
                .column(sr_news_characters_link::Column::CharacterId)
                .filter(sr_news_characters_link::Column::GameId.eq(game_id))
                .filter(sr_news_characters_link::Column::SourceId.eq(source_id))
                .filter(sr_news_characters_link::Column::NewsId.eq(news_id))
                .into_tuple::<String>()
                .all(txn)
                .await
        }
        "zzz" => {
            zzz_news_characters_link::Entity::find()
                .select_only()
                .column(zzz_news_characters_link::Column::CharacterId)
                .filter(zzz_news_characters_link::Column::GameId.eq(game_id))
                .filter(zzz_news_characters_link::Column::SourceId.eq(source_id))
                .filter(zzz_news_characters_link::Column::NewsId.eq(news_id))
                .into_tuple::<String>()
                .all(txn)
                .await
        }
        _ => Ok(Vec::new()),
    }
}

/// 删除一条新闻的既有关联并写入对应游戏的完整角色集合
async fn replace_news_character_links(
    txn: &DatabaseTransaction,
    game_id: &str,
    source_id: &str,
    news_id: &str,
    characters: &[NewsCharacterInput],
) -> Result<(), DbErr> {
    match game_id {
        "ys" => {
            ys_news_characters_link::Entity::delete_many()
                .filter(ys_news_characters_link::Column::GameId.eq(game_id))
                .filter(ys_news_characters_link::Column::SourceId.eq(source_id))
                .filter(ys_news_characters_link::Column::NewsId.eq(news_id))
                .exec(txn)
                .await?;
            for character in characters {
                ys_news_characters_link::ActiveModel {
                    game_id: Set(game_id.to_owned()),
                    source_id: Set(source_id.to_owned()),
                    news_id: Set(news_id.to_owned()),
                    character_id: Set(character.id.clone()),
                    character_collection: Set("character".to_owned()),
                }
                .insert(txn)
                .await?;
            }
        }
        "sr" => {
            sr_news_characters_link::Entity::delete_many()
                .filter(sr_news_characters_link::Column::GameId.eq(game_id))
                .filter(sr_news_characters_link::Column::SourceId.eq(source_id))
                .filter(sr_news_characters_link::Column::NewsId.eq(news_id))
                .exec(txn)
                .await?;
            for character in characters {
                sr_news_characters_link::ActiveModel {
                    game_id: Set(game_id.to_owned()),
                    source_id: Set(source_id.to_owned()),
                    news_id: Set(news_id.to_owned()),
                    character_id: Set(character.id.clone()),
                    character_collection: Set("character".to_owned()),
                }
                .insert(txn)
                .await?;
            }
        }
        "zzz" => {
            zzz_news_characters_link::Entity::delete_many()
                .filter(zzz_news_characters_link::Column::GameId.eq(game_id))
                .filter(zzz_news_characters_link::Column::SourceId.eq(source_id))
                .filter(zzz_news_characters_link::Column::NewsId.eq(news_id))
                .exec(txn)
                .await?;
            for character in characters {
                zzz_news_characters_link::ActiveModel {
                    game_id: Set(game_id.to_owned()),
                    source_id: Set(source_id.to_owned()),
                    news_id: Set(news_id.to_owned()),
                    character_id: Set(character.id.clone()),
                    character_collection: Set("character".to_owned()),
                }
                .insert(txn)
                .await?;
            }
        }
        _ if characters.is_empty() => {}
        _ => {
            return Err(DbErr::Custom(format!(
                "character links are not supported for game {game_id}"
            )));
        }
    }

    Ok(())
}

fn transaction_error(error: TransactionError<DbErr>) -> DbError {
    match error {
        TransactionError::Connection(error) | TransactionError::Transaction(error) => {
            DbError::Query(error)
        }
    }
}
