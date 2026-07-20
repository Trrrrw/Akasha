use serde::Deserialize;
use utoipa::IntoParams;

use akasha_db::repositories::characters::CharListFilter;

const DEFAULT_PAGE_LIMIT: u64 = 20;
const MAX_PAGE_LIMIT: u64 = 100;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct CharacterListQuery {
    pub q: Option<String>,
    pub gender: Option<String>,
    pub cv: Option<String>,
    pub birthday_month: Option<i16>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

impl CharacterListQuery {
    pub(super) fn into_filter(self, game_id: String) -> CharListFilter {
        CharListFilter {
            game_id,
            q: self.q,
            gender: self.gender,
            cv: self.cv,
            birthday_month: self.birthday_month,
            limit: self
                .limit
                .unwrap_or(DEFAULT_PAGE_LIMIT)
                .clamp(1, MAX_PAGE_LIMIT),
            offset: self.offset.unwrap_or(0),
        }
    }
}
