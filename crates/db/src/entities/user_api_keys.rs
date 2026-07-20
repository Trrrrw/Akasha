use sea_orm::entity::prelude::*;

use crate::entities::users;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_api_keys")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(indexed)]
    pub user_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    #[sea_orm(unique)]
    pub key_hash: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub last_used_at: Option<DateTimeWithTimeZone>,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(belongs_to, from = "user_id", to = "id")]
    pub user: HasOne<users::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
