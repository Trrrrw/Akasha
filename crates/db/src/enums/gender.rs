use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::N(16))",
    rename_all = "lowercase"
)]
pub enum Gender {
    Male,
    Female,
    Unknown,
}
