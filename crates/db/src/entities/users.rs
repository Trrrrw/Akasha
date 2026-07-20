use sea_orm::entity::prelude::*;

use crate::entities::{oauth_accounts, user_api_keys, user_groups, user_refresh_tokens};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub disabled_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    #[sea_orm(has_many)]
    pub oauth_accounts: HasMany<oauth_accounts::Entity>,
    #[sea_orm(has_many)]
    pub user_groups: HasMany<user_groups::Entity>,
    #[sea_orm(has_many)]
    pub api_keys: HasMany<user_api_keys::Entity>,
    #[sea_orm(has_many)]
    pub refresh_tokens: HasMany<user_refresh_tokens::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
