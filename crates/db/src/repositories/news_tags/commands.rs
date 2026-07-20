use std::collections::HashSet;

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbErr, EntityTrait, QueryFilter, QueryOrder,
    TransactionError, TransactionTrait,
};

use crate::{
    Db, DbError,
    entities::{news_tags, news_tags_link},
};

pub async fn sync_tags(db: &Db, input: SyncTagsInput) -> Result<SyncTagsResult, DbError> {
    let game_id = input.game_id;
    let source_id = input.source_id;
    let mut tags = input.tags;
    tags.sort_by(tag_order);

    db.conn()
        .transaction::<_, SyncTagsResult, DbErr>(|txn| {
            Box::pin(async move {
                let current = news_tags::Entity::find()
                    .filter(news_tags::Column::GameId.eq(&game_id))
                    .filter(news_tags::Column::SourceId.eq(&source_id))
                    .order_by_asc(news_tags::Column::GroupIndex)
                    .order_by_asc(news_tags::Column::Group)
                    .order_by(news_tags::Column::Index, sea_orm::Order::Asc)
                    .all(txn)
                    .await?
                    .into_iter()
                    .map(NewsTagInput::from)
                    .collect::<Vec<_>>();

                if current == tags {
                    return Ok(SyncTagsResult {
                        changed: false,
                        tags,
                    });
                }

                let names = tags
                    .iter()
                    .map(|tag| tag.name.as_str())
                    .collect::<HashSet<_>>();

                for tag in current
                    .into_iter()
                    .filter(|tag| !names.contains(tag.name.as_str()))
                {
                    news_tags_link::Entity::delete_many()
                        .filter(news_tags_link::Column::GameId.eq(&game_id))
                        .filter(news_tags_link::Column::SourceId.eq(&source_id))
                        .filter(news_tags_link::Column::Name.eq(&tag.name))
                        .exec(txn)
                        .await?;

                    news_tags::Entity::delete_by_id((tag.name, game_id.clone(), source_id.clone()))
                        .exec(txn)
                        .await?;
                }

                for tag in &tags {
                    let key = (tag.name.clone(), game_id.clone(), source_id.clone());
                    if let Some(row) = news_tags::Entity::find_by_id(key).one(txn).await? {
                        let mut active: news_tags::ActiveModel = row.into();
                        active.index = Set(tag.index);
                        active.group = Set(tag.group.clone());
                        active.group_index = Set(tag.group_index);
                        active.update(txn).await?;
                    } else {
                        news_tags::ActiveModel {
                            name: Set(tag.name.clone()),
                            game_id: Set(game_id.clone()),
                            source_id: Set(source_id.clone()),
                            index: Set(tag.index),
                            group: Set(tag.group.clone()),
                            group_index: Set(tag.group_index),
                        }
                        .insert(txn)
                        .await?;
                    }
                }

                Ok(SyncTagsResult {
                    changed: true,
                    tags,
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

pub struct SyncTagsInput {
    pub game_id: String,
    pub source_id: String,
    pub tags: Vec<NewsTagInput>,
}

pub struct SyncTagsResult {
    pub changed: bool,
    pub tags: Vec<NewsTagInput>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewsTagInput {
    pub name: String,
    pub index: i64,
    pub group: Option<String>,
    pub group_index: Option<i64>,
}

impl From<news_tags::Model> for NewsTagInput {
    fn from(value: news_tags::Model) -> Self {
        Self {
            name: value.name,
            index: value.index,
            group: value.group,
            group_index: value.group_index,
        }
    }
}

fn tag_order(left: &NewsTagInput, right: &NewsTagInput) -> std::cmp::Ordering {
    option_some_first(&left.group_index, &right.group_index)
        .then_with(|| option_some_first(&left.group, &right.group))
        .then_with(|| left.index.cmp(&right.index))
        .then_with(|| left.name.cmp(&right.name))
}

fn option_some_first<T: Ord>(left: &Option<T>, right: &Option<T>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
