use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "admin_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,

    pub admin_user_id: i32,
    pub refresh_token_jti: String,

    pub user_agent: Option<String>,
    pub ip_address: Option<String>,

    pub expires_at: DateTimeWithTimeZone,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub last_used_at: Option<DateTimeWithTimeZone>,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    #[sea_orm(belongs_to, from = "admin_user_id", to = "id")]
    pub admin_user: HasOne<super::admin_users::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {}
