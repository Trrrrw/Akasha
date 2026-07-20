use sea_orm::entity::prelude::*;

use crate::{
    entities::games,
    models::{WorkerPhase, WorkerStatus},
};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "worker_states")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(indexed)]
    pub worker_type: String,
    pub source_id: Option<String>,
    pub game_id: String,
    pub phase: WorkerPhase,
    pub status: WorkerStatus,
    #[sea_orm(column_type = "JsonBinary")]
    pub checkpoint: Json,
    pub run_id: Option<String>,
    pub lease_until: Option<DateTimeWithTimeZone>,
    #[sea_orm(column_type = "Text", nullable)]
    pub last_error: Option<String>,
    pub last_success_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    #[sea_orm(belongs_to, from = "game_id", to = "id")]
    pub game: HasOne<games::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
