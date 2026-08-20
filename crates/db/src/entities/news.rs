use sea_orm::entity::prelude::*;

use crate::entities::{games, news_sources, news_tags, sr_game_data, ys_game_data, zzz_game_data};

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
    /// 视频时长，单位为毫秒
    pub video_duration_ms: Option<i64>,
    pub raw_data: Json,

    #[sea_orm(belongs_to, from = "game_id", to = "id")]
    pub game: HasOne<games::Entity>,
    #[sea_orm(belongs_to, from = "(source_id, game_id)", to = "(id, game_id)")]
    pub news_source: HasOne<news_sources::Entity>,

    #[sea_orm(has_many, via = "news_tags_link")]
    pub tags: HasMany<news_tags::Entity>,
    #[sea_orm(has_many, via = "ys_news_characters_link")]
    pub ys_characters: HasMany<ys_game_data::Entity>,
    #[sea_orm(has_many, via = "sr_news_characters_link")]
    pub sr_characters: HasMany<sr_game_data::Entity>,
    #[sea_orm(has_many, via = "zzz_news_characters_link")]
    pub zzz_characters: HasMany<zzz_game_data::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
