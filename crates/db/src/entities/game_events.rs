use sea_orm::entity::prelude::*;

use crate::entities::games;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "game_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub game_id: String,
    #[sea_orm(primary_key)]
    pub id: String,

    pub title: String,
    pub introduction: Option<String>,
    pub main_text: Option<String>,
    pub start: Option<DateTimeWithTimeZone>,
    pub end: Option<DateTimeWithTimeZone>,
    pub tags: Option<Vec<String>>,
    pub url: Option<String>,

    #[sea_orm(belongs_to, from = "game_id", to = "id")]
    pub game: HasOne<games::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
