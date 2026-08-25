use sea_orm::entity::prelude::*;

use crate::entities::{game_events, game_versions, news, news_sources};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "games")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String,
    pub name_en: String,
    pub name_zh: String,
    pub index: i64,
    pub cover: Option<String>,
    pub icon: Option<String>,

    #[sea_orm(has_many)]
    pub news: HasMany<news::Entity>,
    #[sea_orm(has_many)]
    pub news_sources: HasMany<news_sources::Entity>,
    #[sea_orm(has_many)]
    pub events: HasMany<game_events::Entity>,
    #[sea_orm(has_many)]
    pub versions: HasMany<game_versions::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
