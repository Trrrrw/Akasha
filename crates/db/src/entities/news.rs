use sea_orm::entity::prelude::*;

use crate::entities::{games, news_sources, news_tags};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum NewsType {
    #[sea_orm(string_value = "article")]
    Article,

    #[sea_orm(string_value = "video")]
    Video,
}

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "news")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub game_id: String,
    #[sea_orm(primary_key)]
    pub source_id: String,
    #[sea_orm(primary_key)]
    pub id: String,

    pub title: String,
    pub intro: Option<String>,
    pub publish_time: DateTimeWithTimeZone,
    pub source_url: String,
    pub cover: Option<String>,
    pub news_type: NewsType,
    pub video_url: Option<String>,
    #[sea_orm(column_type = "JsonBinary")]
    pub raw_data: Json,

    #[sea_orm(belongs_to, from = "game_id", to = "id")]
    pub game: HasOne<games::Entity>,
    #[sea_orm(belongs_to, from = "(source_id, game_id)", to = "(id, game_id)")]
    pub news_source: HasOne<news_sources::Entity>,

    #[sea_orm(has_many, via = "news_tags_link")]
    pub tags: HasMany<news_tags::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
