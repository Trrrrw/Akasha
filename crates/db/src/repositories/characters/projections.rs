use crate::{entities::characters, models::Gender};

pub struct CharListFilter {
    pub game_id: String,
    /// 关键词，匹配角色名/简介
    pub q: Option<String>,
    /// 性别
    pub gender: Option<String>,
    /// CV
    pub cv: Option<String>,
    /// 生日月份
    pub birthday_month: Option<i16>,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Clone)]
pub struct CharSummary {
    pub id: String,
    pub item_id: String,
    pub name: String,
    pub description: Option<String>,
    pub gender: Option<Gender>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub cv: Option<String>,
}

impl From<characters::Model> for CharSummary {
    fn from(row: characters::Model) -> Self {
        Self {
            id: row.id,
            item_id: row.item_id,
            name: row.name,
            description: row.description,
            gender: row.gender,
            birthday_month: row.birthday_month,
            birthday_day: row.birthday_day,
            cv: row.cv,
        }
    }
}
