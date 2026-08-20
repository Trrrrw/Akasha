use akasha_application::{
    characters::{ZzzCharacter, ZzzCharacterListFilter},
    game_data::GameDataEntry,
};
use sea_orm::{
    ColumnTrait, DbErr, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, sea_query::Expr,
};

use super::json_field;
use crate::{Db, DbError, entities::zzz_game_data, models::text_query_condition};

/// 从绝区零游戏数据的 character 集合读取角色
pub async fn list(
    db: &Db,
    filter: ZzzCharacterListFilter,
) -> Result<(u64, Vec<ZzzCharacter>), DbError> {
    let (total, entries) = list_entries(db, filter).await?;
    let items = entries
        .into_iter()
        .map(|entry| decode(entry.id, entry.summary))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((total, items))
}

/// 按角色专属字段筛选绝区零游戏数据条目
pub async fn list_entries(
    db: &Db,
    filter: ZzzCharacterListFilter,
) -> Result<(u64, Vec<GameDataEntry>), DbError> {
    let field = |path| json_field(Expr::col(zzz_game_data::Column::Summary), path);
    let mut query =
        zzz_game_data::Entity::find().filter(zzz_game_data::Column::Collection.eq("character"));

    if let Some(text_query) = filter.query.as_ref() {
        query = query.filter(text_query_condition(
            text_query,
            &[
                Expr::col(zzz_game_data::Column::Name),
                field("$.name_en"),
                field("$.description"),
                field("$.description_en"),
                field("$.full_name"),
            ],
        ));
    }
    if let Some(specialty_id) = filter.specialty_id {
        query = query.filter(field("$.specialty_id").eq(specialty_id));
    }
    if let Some(specialty) = filter.specialty {
        query = query.filter(field("$.specialty").eq(specialty));
    }
    if let Some(element_id) = filter.element_id {
        query = query.filter(field("$.element_id").eq(element_id));
    }
    if let Some(element) = filter.element {
        query = query.filter(field("$.element").eq(element));
    }
    if let Some(hit_type_id) = filter.hit_type_id {
        query = query.filter(field("$.hit_type_id").eq(hit_type_id));
    }
    if let Some(hit_type) = filter.hit_type {
        query = query.filter(field("$.hit_type").eq(hit_type));
    }
    if let Some(camp_id) = filter.camp_id {
        query = query.filter(field("$.camp_id").eq(camp_id));
    }
    if let Some(camp) = filter.camp {
        query = query.filter(field("$.camp").eq(camp));
    }
    if let Some(rarity) = filter.rarity {
        query = query.filter(field("$.rarity").eq(rarity));
    }
    if let Some(gender) = filter.gender {
        query = query.filter(field("$.gender").eq(gender));
    }
    if let Some(special_element) = filter.special_element {
        query = query.filter(field("$.special_element").eq(special_element));
    }
    if let Some(month) = filter.birthday_month {
        query = query.filter(field("$.birthday_month").eq(month));
    }
    if let Some(day) = filter.birthday_day {
        query = query.filter(field("$.birthday_day").eq(day));
    }
    if filter.birthday_only {
        query = query
            .filter(field("$.birthday_month").is_not_null())
            .filter(field("$.birthday_day").is_not_null());
    }

    let total = query
        .clone()
        .count(db.conn())
        .await
        .map_err(DbError::Query)?;
    let rows = query
        .order_by_asc(zzz_game_data::Column::Name)
        .order_by_asc(zzz_game_data::Column::Id)
        .limit(filter.limit)
        .offset(filter.offset)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;
    Ok((total, rows.into_iter().map(GameDataEntry::from).collect()))
}

fn decode(id: String, summary: serde_json::Value) -> Result<ZzzCharacter, DbError> {
    serde_json::from_value(summary).map_err(|error| {
        DbError::Query(DbErr::Custom(format!(
            "invalid zzz character summary {id}: {error}"
        )))
    })
}
