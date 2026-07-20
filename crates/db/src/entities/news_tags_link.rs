use sea_orm::entity::prelude::*;

use crate::entities::{news, news_tags};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "news_tags_link")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub game_id: String,

    #[sea_orm(primary_key)]
    pub news_id: String,
    #[sea_orm(primary_key)]
    pub source_id: String,

    #[sea_orm(primary_key)]
    pub name: String,

    #[sea_orm(
        belongs_to,
        from = "(game_id, news_id, source_id)",
        to = "(game_id, id, source_id)"
    )]
    pub news: Option<news::Entity>,
    #[sea_orm(
        belongs_to,
        from = "(name, game_id, source_id)",
        to = "(name, game_id, source_id)"
    )]
    pub tag: Option<news_tags::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
