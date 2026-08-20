use sea_orm::entity::prelude::*;

/// 原神游戏数据目录
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ys_game_data")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub collection: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub summary: Json,
    pub detail: Option<Json>,
    pub assets: Json,
    pub raw_data: Option<Json>,
    pub source_hash: Option<String>,
}

impl ActiveModelBehavior for ActiveModel {}
