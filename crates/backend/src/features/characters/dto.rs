use akasha_db::repositories::characters::CharSummary;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[schema(description = "角色信息")]
pub(super) struct CharacterResponse {
    id: String,
    item_id: String,
    name: String,
    description: Option<String>,
    gender: Option<String>,
    birthday_month: Option<i16>,
    birthday_day: Option<i16>,
    cv: Option<String>,
}

impl From<CharSummary> for CharacterResponse {
    fn from(value: CharSummary) -> Self {
        Self {
            id: value.id,
            item_id: value.item_id,
            name: value.name,
            description: value.description,
            gender: value.gender.map(|gender| gender.as_str().to_string()),
            birthday_month: value.birthday_month,
            birthday_day: value.birthday_day,
            cv: value.cv,
        }
    }
}
