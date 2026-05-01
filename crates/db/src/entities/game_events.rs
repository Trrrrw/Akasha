use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "game_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub title: String,
    #[sea_orm(primary_key)]
    pub start: DateTimeWithTimeZone,
    pub end: Option<DateTimeWithTimeZone>,
    pub game_code: String,
    pub desc: Option<String>,
    pub extra: Option<String>,

    #[sea_orm(belongs_to, from = "game_code", to = "game_code")]
    pub game: HasOne<super::games::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {}
