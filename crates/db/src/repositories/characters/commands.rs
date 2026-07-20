use std::collections::HashSet;

use sea_orm::{
    ActiveEnum, ActiveValue::Set, ColumnTrait, Condition, DbErr, EntityTrait, QueryFilter,
    QuerySelect, TransactionError, TransactionTrait, sea_query::OnConflict,
};

use crate::{Db, DbError, entities::characters, models::Gender};

pub struct SyncCharsInput {
    pub game_id: String,
    pub items: Vec<SyncCharInput>,
}

pub struct SyncCharInput {
    pub id: String,
    pub item_id: String,
    pub name: String,
    pub description: Option<String>,
    pub gender: Option<String>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub cv: Option<String>,
    pub extra: serde_json::Value,
}

pub struct SyncCharsResult {
    pub created: u64,
    pub updated: u64,
    pub total: u64,
}

pub async fn sync_chars(db: &Db, input: SyncCharsInput) -> Result<SyncCharsResult, DbError> {
    if input.items.is_empty() {
        return Ok(SyncCharsResult {
            created: 0,
            updated: 0,
            total: 0,
        });
    }
    db.conn()
        .transaction::<_, SyncCharsResult, DbErr>(|txn| {
            Box::pin(async move {
                let mut incoming_keys = HashSet::with_capacity(input.items.len());
                for item in &input.items {
                    let key = (item.id.clone(), item.item_id.clone());
                    if !incoming_keys.insert(key) {
                        return Err(DbErr::Custom(format!(
                            "duplicated character key: id={}, item_id={}",
                            item.id, item.item_id
                        )));
                    }
                }
                let existing_keys = characters::Entity::find()
                    .select_only()
                    .column(characters::Column::Id)
                    .column(characters::Column::ItemId)
                    .filter(characters::Column::GameId.eq(&input.game_id))
                    .into_tuple::<(String, String)>()
                    .all(txn)
                    .await?
                    .into_iter()
                    .collect::<HashSet<_>>();
                let total = input.items.len() as u64;
                let updated = incoming_keys.intersection(&existing_keys).count() as u64;
                let created = total - updated;
                let models = input
                    .items
                    .into_iter()
                    .map(|item| {
                        let gender = match item.gender.as_ref() {
                            Some(gender) => Some(Gender::try_from_value(gender)?),
                            None => None,
                        };
                        Ok(characters::ActiveModel {
                            game_id: Set(input.game_id.clone()),
                            id: Set(item.id),
                            item_id: Set(item.item_id),
                            name: Set(item.name),
                            description: Set(item.description),
                            gender: Set(gender),
                            birthday_month: Set(item.birthday_month),
                            birthday_day: Set(item.birthday_day),
                            cv: Set(item.cv),
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
                let stale_keys = existing_keys
                    .difference(&incoming_keys)
                    .cloned()
                    .collect::<Vec<_>>();
                if !stale_keys.is_empty() {
                    let mut stale_condition = Condition::any();
                    for (id, item_id) in stale_keys {
                        stale_condition = stale_condition.add(
                            Condition::all()
                                .add(characters::Column::Id.eq(id))
                                .add(characters::Column::ItemId.eq(item_id)),
                        );
                    }
                    characters::Entity::delete_many()
                        .filter(characters::Column::GameId.eq(&input.game_id))
                        .filter(stale_condition)
                        .exec(txn)
                        .await?;
                }
                Ok(SyncCharsResult {
                    created,
                    updated,
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
