use sea_orm::entity::prelude::*;

use crate::entities::{characters, news};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "news_characters_link")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub game_id: String,

    #[sea_orm(primary_key)]
    pub news_id: String,
    #[sea_orm(primary_key)]
    pub source_id: String,

    #[sea_orm(primary_key)]
    pub character_id: String,
    #[sea_orm(primary_key)]
    pub character_item_id: String,

    #[sea_orm(
        belongs_to,
        from = "(game_id, news_id, source_id)",
        to = "(game_id, id, source_id)"
    )]
    pub news: Option<news::Entity>,
    #[sea_orm(
        belongs_to,
        from = "(game_id, character_id, character_item_id)",
        to = "(game_id, id, item_id)"
    )]
    pub character: Option<characters::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
