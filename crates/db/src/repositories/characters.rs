use sea_orm::sea_query::{Alias, Expr, Func};

/// 构造 SQLite JSON 字段读取表达式
fn json_field(column: Expr, path: &'static str) -> Expr {
    Expr::expr(Func::cust(Alias::new("json_extract")).arg(column).arg(path))
}

/// 三个独立游戏表共享查询流程，字段差异由各自声明保留
macro_rules! character_repository {
    ($module:ident, $entity:ident, $character:ident, $filter:ident,
     fields [$($field:ident),*], search [$($search:literal),*], voice [$($voice:ident)?]) => {
        mod $module {
            use akasha_application::{characters::{$character, $filter}, game_data::GameDataEntry};
            use sea_orm::{ColumnTrait, DbErr, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter,
                QueryOrder, QuerySelect, sea_query::Expr};
            use crate::{Db, DbError, entities::$entity, models::text_query_condition};

            pub async fn list(db: &Db, filter: $filter) -> Result<(u64, Vec<$character>), DbError> {
                let (total, entries) = list_entries(db, filter).await?;
                let items = entries.into_iter().map(|entry| {
                    serde_json::from_value(entry.summary).map_err(|error| {
                        DbError::Query(DbErr::Custom(format!(
                            "invalid {} character summary {}: {error}", stringify!($module), entry.id)))
                    })
                }).collect::<Result<Vec<_>, _>>()?;
                Ok((total, items))
            }

            pub async fn list_entries(db: &Db, filter: $filter) -> Result<(u64, Vec<GameDataEntry>), DbError> {
                let field = |path| super::json_field(Expr::col($entity::Column::Summary), path);
                let mut query = $entity::Entity::find().filter($entity::Column::Collection.eq("character"));
                if let Some(text) = filter.query.as_ref() {
                    query = query.filter(text_query_condition(text, &[
                        Expr::col($entity::Column::Name), field("$.name_en"),
                        field("$.description"), field("$.description_en"), $(field($search)),*
                    ]));
                }
                $(if let Some(value) = filter.$field {
                    query = query.filter(field(concat!("$.", stringify!($field))).eq(value));
                })*
                $(if let Some(value) = filter.$voice.filter(|value| !value.is_empty()) {
                    query = query.filter(crate::models::literal_contains_condition(value, &[
                        field("$.cv_zh"), field("$.cv_en"), field("$.cv_ja"), field("$.cv_ko")
                    ]));
                })?
                if filter.birthday_only {
                    query = query.filter(field("$.birthday_month").is_not_null())
                        .filter(field("$.birthday_day").is_not_null());
                }
                let total = query.clone().count(db.conn()).await.map_err(DbError::Query)?;
                let rows = query.order_by_asc($entity::Column::Name).order_by_asc($entity::Column::Id)
                    .limit(filter.limit).offset(filter.offset).all(db.conn()).await.map_err(DbError::Query)?;
                Ok((total, rows.into_iter().map(GameDataEntry::from).collect()))
            }
        }
    };
}

character_repository!(ys, ys_game_data, YsCharacter, YsCharacterListFilter,
    fields [element, weapon_type, rarity, region, affiliation, birthday_month, birthday_day, special],
    search [], voice [voice_actor]);
character_repository!(sr, sr_game_data, SrCharacter, SrCharacterListFilter,
    fields [path, combat_type, rarity, camp, birthday_month, birthday_day],
    search [], voice [voice_actor]);
character_repository!(zzz, zzz_game_data, ZzzCharacter, ZzzCharacterListFilter,
    fields [specialty_id, specialty, element_id, element, hit_type_id, hit_type, camp_id, camp,
        rarity, gender, special_element, birthday_month, birthday_day],
    search ["$.full_name"], voice []);

pub(crate) use sr::{list as list_sr, list_entries as list_sr_entries};
pub(crate) use ys::{list as list_ys, list_entries as list_ys_entries};
pub(crate) use zzz::{list as list_zzz, list_entries as list_zzz_entries};
