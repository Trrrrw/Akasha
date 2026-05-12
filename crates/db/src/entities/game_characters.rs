use sea_orm::entity::prelude::*;

use crate::enums::gender::Gender;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "game_characters")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub character_code: String,
    #[sea_orm(primary_key)]
    pub game_code: String,
    pub name: String,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub release_time: Option<DateTimeWithTimeZone>,
    pub gender: Option<Gender>,
    pub extra: Option<String>,

    #[sea_orm(belongs_to, from = "game_code", to = "game_code")]
    pub game: HasOne<super::games::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {}
