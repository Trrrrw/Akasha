use akasha_application::{
    characters::{SrCharacter, SrCharacterListFilter},
    game_data::GameDataEntry,
};
use sea_orm::{
    ColumnTrait, DbErr, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, sea_query::Expr,
};

use super::json_field;
use crate::{
    Db, DbError,
    entities::sr_game_data,
    models::{literal_contains_condition, text_query_condition},
};

/// 从星铁游戏数据的 character 集合读取角色
pub async fn list(
    db: &Db,
    filter: SrCharacterListFilter,
) -> Result<(u64, Vec<SrCharacter>), DbError> {
    let (total, entries) = list_entries(db, filter).await?;
    let items = entries
        .into_iter()
        .map(|entry| decode(entry.id, entry.summary))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((total, items))
}

/// 按角色专属字段筛选星铁游戏数据条目
pub async fn list_entries(
    db: &Db,
    filter: SrCharacterListFilter,
) -> Result<(u64, Vec<GameDataEntry>), DbError> {
    let field = |path| json_field(Expr::col(sr_game_data::Column::Summary), path);
    let mut query =
        sr_game_data::Entity::find().filter(sr_game_data::Column::Collection.eq("character"));

    if let Some(text_query) = filter.query.as_ref() {
        query = query.filter(text_query_condition(
            text_query,
            &[
                Expr::col(sr_game_data::Column::Name),
                field("$.name_en"),
                field("$.description"),
                field("$.description_en"),
            ],
        ));
    }
    if let Some(path) = filter.path {
        query = query.filter(field("$.path").eq(path));
    }
    if let Some(combat_type) = filter.combat_type {
        query = query.filter(field("$.combat_type").eq(combat_type));
    }
    if let Some(rarity) = filter.rarity {
        query = query.filter(field("$.rarity").eq(rarity));
    }
    if let Some(camp) = filter.camp {
        query = query.filter(field("$.camp").eq(camp));
    }
    if let Some(voice_actor) = non_empty(filter.voice_actor) {
        query = query.filter(literal_contains_condition(
            voice_actor,
            &[
                field("$.cv_zh"),
                field("$.cv_en"),
                field("$.cv_ja"),
                field("$.cv_ko"),
            ],
        ));
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
        .order_by_asc(sr_game_data::Column::Name)
        .order_by_asc(sr_game_data::Column::Id)
        .limit(filter.limit)
        .offset(filter.offset)
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;
    Ok((total, rows.into_iter().map(GameDataEntry::from).collect()))
}

fn decode(id: String, summary: serde_json::Value) -> Result<SrCharacter, DbError> {
    serde_json::from_value(summary).map_err(|error| {
        DbError::Query(DbErr::Custom(format!(
            "invalid sr character summary {id}: {error}"
        )))
    })
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}
