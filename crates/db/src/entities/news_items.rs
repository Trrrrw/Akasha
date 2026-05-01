use sea_orm::{IntoActiveModel, Order, QueryOrder, entity::prelude::*};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "news_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub remote_id: String,
    #[sea_orm(primary_key)]
    pub game_code: String,
    #[sea_orm(primary_key)]
    pub source: String,
    pub title: String,
    pub intro: Option<String>,
    pub publish_time: DateTimeWithTimeZone,
    pub source_url: String,
    pub cover: String,
    pub is_video: bool,
    pub video_url: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub raw_data: String,

    #[sea_orm(belongs_to, from = "game_code", to = "game_code")]
    pub game: HasOne<super::games::Entity>,
    #[sea_orm(belongs_to, from = "source", to = "name")]
    pub news_source: HasOne<super::news_sources::Entity>,
    #[sea_orm(has_many, via = "news_tags_link")]
    pub tags: HasMany<super::tags::Entity>,
    #[sea_orm(has_many, via = "news_categories_link")]
    pub categories: HasMany<super::news_categories::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub async fn get_by_pk(
        remote_id: &str,
        game_code: &str,
        source: &str,
    ) -> Result<Option<Model>, sea_orm::DbErr> {
        let conn = crate::pool();

        Self::find_by_id((
            remote_id.to_string(),
            game_code.to_string(),
            source.to_string(),
        ))
        .one(conn)
        .await
    }

    pub async fn create_if_not_exists(new_news_item: Model) -> Result<Model, sea_orm::DbErr> {
        let conn = crate::pool();
        let remote_id = new_news_item.remote_id.clone();
        let game_code = new_news_item.game_code.clone();
        let source = new_news_item.source.clone();

        Self::insert(new_news_item.into_active_model())
            .on_conflict_do_nothing()
            .exec_without_returning(conn)
            .await?;

        Self::find_by_id((remote_id, game_code, source))
            .one(conn)
            .await?
            .ok_or(sea_orm::DbErr::RecordNotFound(
                "news item should exist after create_if_not_exists".to_string(),
            ))
    }

    pub async fn get_local_latest_news(
        source: Option<&str>,
        game: Option<String>,
    ) -> Result<Option<Model>, sea_orm::DbErr> {
        let conn = crate::pool();
        let mut query = Entity::find();

        if let Some(game) = game {
            query = query.filter(Column::GameCode.eq(game));
        }

        if let Some(source) = source {
            query = query.filter(Column::Source.eq(source));
        }

        query
            .order_by(Column::PublishTime, Order::Desc)
            .one(conn)
            .await
    }
}
