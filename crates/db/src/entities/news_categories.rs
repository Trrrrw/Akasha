use sea_orm::{IntoActiveModel, entity::prelude::*};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "news_categories")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub title: String,
    #[sea_orm(primary_key)]
    pub game_code: String,
    pub index: i32,

    #[sea_orm(belongs_to, from = "game_code", to = "game_code")]
    pub game: HasOne<super::games::Entity>,
    #[sea_orm(has_many, via = "news_categories_link")]
    pub news: HasMany<super::news_items::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub async fn create_if_not_exists(category: Model) -> Result<(), sea_orm::DbErr> {
        let conn = crate::pool();

        Self::insert(category.into_active_model())
            .on_conflict_do_nothing()
            .exec_without_returning(conn)
            .await?;

        Ok(())
    }
}
