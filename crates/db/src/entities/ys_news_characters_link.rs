use sea_orm::entity::prelude::*;

use crate::entities::{news, ys_game_data};

/// 原神新闻与角色的关联
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "ys_news_characters_link")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub game_id: String,
    #[sea_orm(primary_key)]
    pub source_id: String,
    #[sea_orm(primary_key)]
    pub news_id: String,
    #[sea_orm(primary_key)]
    pub character_id: String,
    #[sea_orm(default_value = "character")]
    pub character_collection: String,

    #[sea_orm(
        belongs_to,
        from = "(game_id, news_id, source_id)",
        to = "(game_id, id, source_id)"
    )]
    pub news: Option<news::Entity>,
    #[sea_orm(
        belongs_to,
        from = "(character_collection, character_id)",
        to = "(collection, id)"
    )]
    pub character: Option<ys_game_data::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
