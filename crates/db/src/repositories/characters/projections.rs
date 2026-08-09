use crate::entities::characters;

pub(crate) use akasha_application::characters::{CharacterListFilter, CharacterSummary};

impl From<characters::Model> for CharacterSummary {
    /// 将角色 Entity 映射为应用层角色读取模型
    fn from(row: characters::Model) -> Self {
        Self {
            id: row.id,
            item_id: row.item_id,
            name: row.name,
            description: row.description,
            gender: row.gender.map(|gender| gender.as_str().to_owned()),
            birthday_month: row.birthday_month,
            birthday_day: row.birthday_day,
            voice_actor: row.cv,
        }
    }
}
