use sea_orm::entity::prelude::*;

use crate::entities::{games, news, news_tags};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "news_sources")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String,
    #[sea_orm(primary_key)]
    pub game_id: String,

    pub name: String,
    pub index: i64,

    #[sea_orm(belongs_to, from = "game_id", to = "id")]
    pub game: HasOne<games::Entity>,

    #[sea_orm(has_many)]
    pub news: HasMany<news::Entity>,
    #[sea_orm(has_many)]
    pub tags: HasMany<news_tags::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
