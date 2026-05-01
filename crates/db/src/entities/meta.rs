use sea_orm::{IntoActiveModel, entity::prelude::*};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "meta")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub key: String,
    pub value: String,
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub async fn create_if_not_exists(meta: Model) -> Result<(), sea_orm::DbErr> {
        let conn = crate::pool();

        Self::insert(meta.into_active_model())
            .on_conflict_do_nothing()
            .exec_without_returning(conn)
            .await?;

        Ok(())
    }

    pub async fn get_value_by_key(key: &str) -> Result<Option<String>, sea_orm::DbErr> {
        let conn = crate::pool();

        let meta = Self::find_by_id(key.to_string()).one(conn).await?;

        Ok(meta.map(|meta| meta.value))
    }

    pub async fn set_value(key: &str, value: String) -> Result<(), sea_orm::DbErr> {
        let conn = crate::pool();

        if let Some(mut meta) = Self::find_by_id(key.to_string()).one(conn).await? {
            meta.value = value;
            let active_model: ActiveModel = meta.into();
            active_model.update(conn).await?;
        } else {
            Self::insert(
                Model {
                    key: key.to_string(),
                    value,
                }
                .into_active_model(),
            )
            .exec_without_returning(conn)
            .await?;
        }

        Ok(())
    }
}
