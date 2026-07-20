use sea_orm::entity::prelude::*;

use crate::entities::users;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_refresh_tokens")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(indexed)]
    pub user_id: Uuid,
    #[sea_orm(unique)]
    pub token_hash: String,
    pub expires_at: DateTimeWithTimeZone,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub replaced_by_token_id: Option<Uuid>,
    pub created_at: DateTimeWithTimeZone,
    pub last_used_at: Option<DateTimeWithTimeZone>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,

    #[sea_orm(belongs_to, from = "user_id", to = "id")]
    pub user: HasOne<users::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
