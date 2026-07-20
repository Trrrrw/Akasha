use sea_orm::{
    ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::{Db, DbError, entities::characters, models::TitleQuery};

use super::projections::{CharListFilter, CharSummary};

pub async fn get_char_list(
    db: &Db,
    filter: CharListFilter,
) -> Result<(u64, Vec<CharSummary>), DbError> {
    let mut query =
        characters::Entity::find().filter(characters::Column::GameId.eq(&filter.game_id));
    if let Some(q) = filter.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        let title_query = TitleQuery::new(q);
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
    if let Some(cv) = filter
        .cv
        .as_deref()
        .map(str::trim)
        .filter(|cv| !cv.is_empty())
    {
        query = query.filter(characters::Column::Cv.contains(cv));
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
        .limit(filter.limit)
        .offset(filter.offset)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;
    Ok((total, rows.into_iter().map(CharSummary::from).collect()))
}
