use std::collections::{HashMap, HashSet};

use akasha_application::characters::{SyncCharactersCommand, SyncCharactersResult};
use sea_orm::{
    ActiveEnum, ActiveValue::Set, ColumnTrait, Condition, DbErr, EntityTrait, QueryFilter,
    TransactionError, TransactionTrait, sea_query::OnConflict,
};
use serde_json::json;

use crate::{
    Db, DbError,
    entities::{characters, news_characters_link},
    models::Gender,
    repositories::audit,
};

/// 在事务中同步一个游戏的角色目录
pub async fn sync_characters(
    db: &Db,
    command: SyncCharactersCommand,
) -> Result<SyncCharactersResult, DbError> {
    if command.items.is_empty() {
        return Ok(SyncCharactersResult {
            created: 0,
            updated: 0,
            deleted: 0,
            changed: false,
            total: 0,
        });
    }
    db.conn()
        .transaction::<_, SyncCharactersResult, DbErr>(|txn| {
            Box::pin(async move {
                // 校验输入主键在本次同步中唯一，并预加载现有主键
                let mut incoming_keys = HashSet::with_capacity(command.items.len());
                for item in &command.items {
                    let key = (item.id.clone(), item.item_id.clone());
                    if !incoming_keys.insert(key) {
                        return Err(DbErr::Custom(format!(
                            "duplicated character key: id={}, item_id={}",
                            item.id, item.item_id
                        )));
                    }
                }
                let existing_rows = characters::Entity::find()
                    .filter(characters::Column::GameId.eq(&command.game_id))
                    .all(txn)
                    .await?;
                let existing_keys = existing_rows
                    .iter()
                    .map(|row| (row.id.clone(), row.item_id.clone()))
                    .collect::<HashSet<_>>();
                let incoming_values = command
                    .items
                    .iter()
                    .map(|item| {
                        let gender = match item.gender.as_ref() {
                            Some(gender) => Some(Gender::try_from_value(gender)?),
                            None => None,
                        };
                        Ok::<_, DbErr>((
                            (item.id.clone(), item.item_id.clone()),
                            (
                                item.name.clone(),
                                item.description.clone(),
                                gender,
                                item.birthday_month,
                                item.birthday_day,
                                item.voice_actor.clone(),
                                item.extra.clone(),
                            ),
                        ))
                    })
                    .collect::<Result<HashMap<_, _>, DbErr>>()?;
                let total = command.items.len() as u64;
                let updated = incoming_keys.intersection(&existing_keys).count() as u64;
                let created = total - updated;
                let changed = existing_rows.len() != incoming_values.len()
                    || existing_rows.iter().any(|row| {
                        let key = (row.id.clone(), row.item_id.clone());
                        let Some((
                            name,
                            description,
                            gender,
                            birthday_month,
                            birthday_day,
                            voice_actor,
                            extra,
                        )) = incoming_values.get(&key)
                        else {
                            return true;
                        };
                        row.name != *name
                            || row.description != *description
                            || row.gender != *gender
                            || row.birthday_month != *birthday_month
                            || row.birthday_day != *birthday_day
                            || row.cv != *voice_actor
                            || row.extra != *extra
                    });

                // 转换并批量 upsert 本次目录中的角色
                let models = command
                    .items
                    .into_iter()
                    .map(|item| {
                        let gender = match item.gender.as_ref() {
                            Some(gender) => Some(Gender::try_from_value(gender)?),
                            None => None,
                        };
                        Ok(characters::ActiveModel {
                            game_id: Set(command.game_id.clone()),
                            id: Set(item.id),
                            item_id: Set(item.item_id),
                            name: Set(item.name),
                            description: Set(item.description),
                            gender: Set(gender),
                            birthday_month: Set(item.birthday_month),
                            birthday_day: Set(item.birthday_day),
                            cv: Set(item.voice_actor),
                            extra: Set(item.extra),
                        })
                    })
                    .collect::<Result<Vec<_>, DbErr>>()?;
                for chunk in models.chunks(100) {
                    characters::Entity::insert_many(chunk.iter().cloned())
                        .on_conflict(
                            OnConflict::columns([
                                characters::Column::GameId,
                                characters::Column::Id,
                                characters::Column::ItemId,
                            ])
                            .update_column(characters::Column::Name)
                            .update_column(characters::Column::Description)
                            .update_column(characters::Column::Gender)
                            .update_column(characters::Column::BirthdayMonth)
                            .update_column(characters::Column::BirthdayDay)
                            .update_column(characters::Column::Cv)
                            .update_column(characters::Column::Extra)
                            .to_owned(),
                        )
                        .exec(txn)
                        .await?;
                }

                // 删除数据库中已不在本次完整目录里的角色
                let stale_keys = existing_keys
                    .difference(&incoming_keys)
                    .cloned()
                    .collect::<Vec<_>>();
                let deleted = stale_keys.len() as u64;
                if !stale_keys.is_empty() {
                    let mut stale_link_condition = Condition::any();
                    for (id, item_id) in &stale_keys {
                        stale_link_condition = stale_link_condition.add(
                            Condition::all()
                                .add(news_characters_link::Column::CharacterId.eq(id))
                                .add(news_characters_link::Column::CharacterItemId.eq(item_id)),
                        );
                    }
                    news_characters_link::Entity::delete_many()
                        .filter(news_characters_link::Column::GameId.eq(&command.game_id))
                        .filter(stale_link_condition)
                        .exec(txn)
                        .await?;

                    let mut stale_condition = Condition::any();
                    for (id, item_id) in stale_keys {
                        stale_condition = stale_condition.add(
                            Condition::all()
                                .add(characters::Column::Id.eq(id))
                                .add(characters::Column::ItemId.eq(item_id)),
                        );
                    }
                    characters::Entity::delete_many()
                        .filter(characters::Column::GameId.eq(&command.game_id))
                        .filter(stale_condition)
                        .exec(txn)
                        .await?;
                }

                // 目录同步会覆盖现有字段并清理过期角色，因此记录一次完整操作
                audit::insert(
                    txn,
                    &command.audit,
                    "characters.sync",
                    Some("game"),
                    Some(command.game_id.clone()),
                    json!({
                        "created": created,
                        "updated": updated,
                        "deleted": deleted,
                        "changed": changed,
                        "total": total,
                    }),
                )
                .await?;

                Ok(SyncCharactersResult {
                    created,
                    updated,
                    deleted,
                    changed,
                    total,
                })
            })
        })
        .await
        .map_err(|err| match err {
            TransactionError::Connection(err) | TransactionError::Transaction(err) => {
                DbError::Query(err)
            }
        })
}
