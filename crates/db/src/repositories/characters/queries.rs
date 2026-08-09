use sea_orm::{
    ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::{Db, DbError, entities::characters, models::TitleQuery};

use super::projections::{CharacterListFilter, CharacterSummary};

/// 列出符合给定应用层筛选条件的角色
pub async fn list_characters(
    db: &Db,
    filter: CharacterListFilter,
) -> Result<(u64, Vec<CharacterSummary>), DbError> {
    let mut query =
        characters::Entity::find().filter(characters::Column::GameId.eq(&filter.game_id));
    if let Some(search_term) = filter
        .query
        .as_deref()
        .map(str::trim)
        .filter(|search_term| !search_term.is_empty())
    {
        let title_query = TitleQuery::new(search_term);
        for keyword in title_query.includes {
            query = query.filter(
                Condition::any()
                    .add(characters::Column::Name.contains(&keyword))
                    .add(characters::Column::Description.contains(&keyword)),
            );
        }
        for keyword in title_query.excludes {
            query = query.filter(
                Condition::all()
                    .add(characters::Column::Name.not_like(format!("%{}%", keyword)))
                    .add(characters::Column::Description.not_like(format!("%{}%", keyword))),
            );
        }
    }
    if let Some(gender) = filter.gender {
        query = query.filter(characters::Column::Gender.eq(gender));
    }
    if let Some(voice_actor) = filter
        .voice_actor
        .as_deref()
        .map(str::trim)
        .filter(|voice_actor| !voice_actor.is_empty())
    {
        query = query.filter(characters::Column::Cv.contains(voice_actor));
    }
    if let Some(month) = filter.birthday_month {
        query = query.filter(characters::Column::BirthdayMonth.eq(month));
    }
    let total = query
        .clone()
        .count(db.conn())
        .await
        .map_err(DbError::Query)?;
    let rows = query
        .order_by(characters::Column::Name, sea_orm::Order::Asc)
        .order_by_asc(characters::Column::Id)
        .order_by_asc(characters::Column::ItemId)
        .limit(filter.limit)
        .offset(filter.offset)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;
    Ok((
        total,
        rows.into_iter().map(CharacterSummary::from).collect(),
    ))
}
