use sea_orm::sea_query::{Alias, Expr, Func};

/// 数据表和角色模型按游戏独立，共用分页和生日条件
macro_rules! character_repository {
    ($module:ident, $entity:ident, $character:ident, $filter:ident) => {
        mod $module {
            use crate::{Db, DbError, entities::$entity};
            use akasha_application::{
                characters::{$character, $filter},
                game_data::GameDataEntry,
            };
            use sea_orm::{
                ColumnTrait, DbErr, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter,
                QueryOrder, QuerySelect,
            };
            pub async fn list(db: &Db, filter: $filter) -> Result<(u64, Vec<$character>), DbError> {
                let mut query =
                    $entity::Entity::find().filter($entity::Column::Collection.eq("character"));
                if filter.birthday_only {
                    for path in ["$.birthday_month", "$.birthday_day"] {
                        query = query.filter(
                            super::Expr::expr(
                                super::Func::cust(super::Alias::new("json_extract"))
                                    .arg(super::Expr::col($entity::Column::Summary))
                                    .arg(path),
                            )
                            .is_not_null(),
                        );
                    }
                }
                let total = query
                    .clone()
                    .count(db.conn())
                    .await
                    .map_err(DbError::Query)?;
                let rows = query
                    .order_by_asc($entity::Column::Name)
                    .order_by_asc($entity::Column::Id)
                    .limit(filter.limit)
                    .offset(filter.offset)
                    .all(db.conn())
                    .await
                    .map_err(DbError::Query)?;
                let items = rows
                    .into_iter()
                    .map(GameDataEntry::from)
                    .map(|entry| {
                        let mut item: $character =
                            serde_json::from_value(entry.summary).map_err(|error| {
                                DbError::Query(DbErr::Custom(format!(
                                    "invalid {} character {}: {error}",
                                    stringify!($module),
                                    entry.id
                                )))
                            })?;
                        item.icon_url = entry.icon.or(item.icon_url).or_else(|| {
                            entry
                                .assets
                                .get("icon")
                                .and_then(|v| v.as_str())
                                .map(str::to_owned)
                        });
                        Ok(item)
                    })
                    .collect::<Result<Vec<_>, DbError>>()?;
                Ok((total, items))
            }
        }
    };
}
character_repository!(ys, ys_game_data, YsCharacter, YsCharacterListFilter);
character_repository!(sr, sr_game_data, SrCharacter, SrCharacterListFilter);
character_repository!(zzz, zzz_game_data, ZzzCharacter, ZzzCharacterListFilter);
pub(crate) use sr::list as list_sr;
pub(crate) use ys::list as list_ys;
pub(crate) use zzz::list as list_zzz;
