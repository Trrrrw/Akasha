use sea_orm::{IntoActiveModel, entity::prelude::*};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "news_categories_link")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub news_remote_id: String,
    #[sea_orm(primary_key)]
    pub news_game_belong: String,
    #[sea_orm(primary_key)]
    pub news_source_belong: String,
    #[sea_orm(primary_key)]
    pub category_title: String,
    #[sea_orm(primary_key)]
    pub category_game_belong: String,

    #[sea_orm(
        belongs_to,
        from = "(news_remote_id, news_game_belong, news_source_belong)",
        to = "(remote_id, game_code, source)"
    )]
    pub news: Option<super::news_items::Entity>,
    #[sea_orm(
        belongs_to,
        from = "(category_title, category_game_belong)",
        to = "(title, game_code)"
    )]
    pub tag: Option<super::news_categories::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub async fn create_if_not_exists(link: Model) -> Result<(), sea_orm::DbErr> {
        let conn = crate::pool();

        Self::insert(link.into_active_model())
            .on_conflict_do_nothing()
            .exec_without_returning(conn)
            .await?;

        Ok(())
    }
}
