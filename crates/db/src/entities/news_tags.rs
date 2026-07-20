use sea_orm::entity::prelude::*;

use crate::entities::{news, news_sources};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "news_tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub name: String,
    #[sea_orm(primary_key)]
    pub game_id: String,
    #[sea_orm(primary_key)]
    pub source_id: String,
    pub index: i64,

    pub group: Option<String>,
    pub group_index: Option<i64>,

    #[sea_orm(belongs_to, from = "(source_id, game_id)", to = "(id, game_id)")]
    pub source: HasOne<news_sources::Entity>,

    #[sea_orm(has_many, via = "news_tags_link")]
    pub news: HasMany<news::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
