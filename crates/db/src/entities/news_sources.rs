use sea_orm::{IntoActiveModel, entity::prelude::*};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "news_sources")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub name: String,
    pub display_name: String,
    pub description: String,

    #[sea_orm(has_many)]
    pub news: HasMany<super::news_items::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub async fn create_if_not_exists(source: Model) -> Result<(), sea_orm::DbErr> {
        let conn = crate::pool();

        Self::insert(source.into_active_model())
            .on_conflict_do_nothing()
            .exec_without_returning(conn)
            .await?;

        Ok(())
    }
}
