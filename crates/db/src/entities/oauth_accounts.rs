use sea_orm::entity::prelude::*;

use crate::entities::users;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "oauth_accounts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub provider: String,
    #[sea_orm(primary_key)]
    pub provider_user_id: String,
    #[sea_orm(indexed)]
    pub user_id: Uuid,
    pub provider_login: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    #[sea_orm(belongs_to, from = "user_id", to = "id")]
    pub user: HasOne<users::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
