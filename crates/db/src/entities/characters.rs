use sea_orm::entity::prelude::*;

use crate::{entities::games, models::Gender};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "game_characters")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub game_id: String,
    #[sea_orm(primary_key)]
    pub id: String,
    #[sea_orm(primary_key)]
    pub item_id: String,

    pub name: String,
    pub description: Option<String>,
    pub gender: Option<Gender>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub cv: Option<String>,

    #[sea_orm(column_type = "JsonBinary")]
    pub extra: Json,

    #[sea_orm(belongs_to, from = "game_id", to = "id")]
    pub game: HasOne<games::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
