use sea_orm::entity::prelude::*;

use crate::{entities::users, models};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_groups")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub user_id: Uuid,
    #[sea_orm(primary_key)]
    pub group: models::UserGroup,
    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(belongs_to, from = "user_id", to = "id")]
    pub user: HasOne<users::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
