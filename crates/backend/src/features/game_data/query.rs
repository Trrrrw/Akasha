use akasha_application::{
    characters::{SrCharacterListFilter, YsCharacterListFilter, ZzzCharacterListFilter},
    game_data::{GameDataCollectionFilter, GameDataListFilter},
    search::TextQuery,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::http::error::AppError;

const DEFAULT_PAGE_LIMIT: u64 = 20;
const MAX_PAGE_LIMIT: u64 = 100;

/// 游戏数据列表查询参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct GameDataListQuery {
    /// 名称和摘要查询，支持空格 AND、竖线 OR、减号排除和引号短语
    q: Option<String>,
    /// 角色元素；原神、绝区零 character 集合可用
    element: Option<String>,
    /// 角色武器类型；原神 character 集合可用
    weapon_type: Option<String>,
    /// 角色稀有度；character 集合可用
    rarity: Option<String>,
    /// 角色地区；原神 character 集合可用
    region: Option<String>,
    /// 角色所属组织；原神 character 集合可用
    affiliation: Option<String>,
    /// 任一语言配音演员关键词；原神、星铁 character 集合可用
    cv: Option<String>,
    /// 角色命途；星铁 character 集合可用
    path: Option<String>,
    /// 角色战斗属性；星铁 character 集合可用
    combat_type: Option<String>,
    /// 角色阵营；星铁、绝区零 character 集合可用
    camp: Option<String>,
    /// 角色特性 ID；绝区零 character 集合可用
    specialty_id: Option<i32>,
    /// 角色特性；绝区零 character 集合可用
    specialty: Option<String>,
    /// 角色属性 ID；绝区零 character 集合可用
    element_id: Option<i32>,
    /// 角色攻击类型 ID；绝区零 character 集合可用
    hit_type_id: Option<i32>,
    /// 角色攻击类型；绝区零 character 集合可用
    hit_type: Option<String>,
    /// 角色阵营 ID；绝区零 character 集合可用
    camp_id: Option<i32>,
    /// 角色性别；绝区零 character 集合仅支持 male 或 female
    gender: Option<String>,
    /// 角色特殊属性；绝区零 character 集合可用
    special_element: Option<String>,
    /// 是否为特殊目录角色；原神 character 集合可用
    special: Option<bool>,
    /// 角色生日月份；character 集合可用，取值 1 到 12
    birthday_month: Option<i16>,
    /// 角色生日日期；character 集合可用，取值 1 到 31
    birthday_day: Option<i16>,
    /// 每页数量，默认 20，最大 100
    limit: Option<u64>,
    /// 分页偏移，默认 0
    offset: Option<u64>,
}

impl GameDataListQuery {
    pub(super) fn into_filter(
        self,
        game_id: String,
        collection: String,
    ) -> Result<GameDataListFilter, AppError> {
        self.validate(&game_id, &collection)?;
        let query = parse_query(self.q)?;
        let limit = self
            .limit
            .unwrap_or(DEFAULT_PAGE_LIMIT)
            .clamp(1, MAX_PAGE_LIMIT);
        let offset = self.offset.unwrap_or(0);
        let collection_filter = if collection == "character" {
            Some(match game_id.as_str() {
                "ys" => GameDataCollectionFilter::YsCharacter(YsCharacterListFilter {
                    query: query.clone(),
                    element: non_empty(self.element),
                    weapon_type: non_empty(self.weapon_type),
                    rarity: non_empty(self.rarity),
                    region: non_empty(self.region),
                    affiliation: non_empty(self.affiliation),
                    voice_actor: non_empty(self.cv),
                    birthday_month: self.birthday_month,
                    birthday_day: self.birthday_day,
                    special: self.special,
                    birthday_only: false,
                    limit,
                    offset,
                }),
                "sr" => GameDataCollectionFilter::SrCharacter(SrCharacterListFilter {
                    query: query.clone(),
                    path: non_empty(self.path),
                    combat_type: non_empty(self.combat_type),
                    rarity: non_empty(self.rarity),
                    camp: non_empty(self.camp),
                    voice_actor: non_empty(self.cv),
                    birthday_month: self.birthday_month,
                    birthday_day: self.birthday_day,
                    birthday_only: false,
                    limit,
                    offset,
                }),
                "zzz" => GameDataCollectionFilter::ZzzCharacter(ZzzCharacterListFilter {
                    query: query.clone(),
                    specialty_id: self.specialty_id,
                    specialty: non_empty(self.specialty),
                    element_id: self.element_id,
                    element: non_empty(self.element),
                    hit_type_id: self.hit_type_id,
                    hit_type: non_empty(self.hit_type),
                    camp_id: self.camp_id,
                    camp: non_empty(self.camp),
                    rarity: self
                        .rarity
                        .map(|value| parse_i16("rarity", &value))
                        .transpose()?,
                    gender: non_empty(self.gender),
                    special_element: non_empty(self.special_element),
                    birthday_month: self.birthday_month,
                    birthday_day: self.birthday_day,
                    birthday_only: false,
                    limit,
                    offset,
                }),
                _ => unreachable!("game id was validated"),
            })
        } else {
            None
        };

        Ok(GameDataListFilter {
            game_id,
            collection,
            query,
            collection_filter,
            limit,
            offset,
        })
    }

    fn validate(&self, game_id: &str, collection: &str) -> Result<(), AppError> {
        validate_birthday(self.birthday_month, self.birthday_day)?;
        if collection != "character" {
            if self.has_character_filter() {
                return Err(AppError::BadRequest(
                    "character filters require the character collection".to_owned(),
                ));
            }
            return Ok(());
        }

        let unsupported = match game_id {
            "ys" => first_present(&[
                ("path", self.path.is_some()),
                ("combat_type", self.combat_type.is_some()),
                ("camp", self.camp.is_some()),
                ("specialty_id", self.specialty_id.is_some()),
                ("specialty", self.specialty.is_some()),
                ("element_id", self.element_id.is_some()),
                ("hit_type_id", self.hit_type_id.is_some()),
                ("hit_type", self.hit_type.is_some()),
                ("camp_id", self.camp_id.is_some()),
                ("gender", self.gender.is_some()),
                ("special_element", self.special_element.is_some()),
            ]),
            "sr" => first_present(&[
                ("element", self.element.is_some()),
                ("weapon_type", self.weapon_type.is_some()),
                ("region", self.region.is_some()),
                ("affiliation", self.affiliation.is_some()),
                ("specialty_id", self.specialty_id.is_some()),
                ("specialty", self.specialty.is_some()),
                ("element_id", self.element_id.is_some()),
                ("hit_type_id", self.hit_type_id.is_some()),
                ("hit_type", self.hit_type.is_some()),
                ("camp_id", self.camp_id.is_some()),
                ("gender", self.gender.is_some()),
                ("special_element", self.special_element.is_some()),
                ("special", self.special.is_some()),
            ]),
            "zzz" => first_present(&[
                ("weapon_type", self.weapon_type.is_some()),
                ("region", self.region.is_some()),
                ("affiliation", self.affiliation.is_some()),
                ("cv", self.cv.is_some()),
                ("path", self.path.is_some()),
                ("combat_type", self.combat_type.is_some()),
                ("special", self.special.is_some()),
            ]),
            _ => None,
        };
        if let Some(field) = unsupported {
            return Err(AppError::BadRequest(format!(
                "{field} is not supported for {game_id} characters"
            )));
        }
        if !matches!(self.gender.as_deref(), None | Some("male" | "female")) {
            return Err(AppError::BadRequest(
                "gender must be male or female".to_owned(),
            ));
        }
        Ok(())
    }

    fn has_character_filter(&self) -> bool {
        self.element.is_some()
            || self.weapon_type.is_some()
            || self.rarity.is_some()
            || self.region.is_some()
            || self.affiliation.is_some()
            || self.cv.is_some()
            || self.path.is_some()
            || self.combat_type.is_some()
            || self.camp.is_some()
            || self.specialty_id.is_some()
            || self.specialty.is_some()
            || self.element_id.is_some()
            || self.hit_type_id.is_some()
            || self.hit_type.is_some()
            || self.camp_id.is_some()
            || self.gender.is_some()
            || self.special_element.is_some()
            || self.special.is_some()
            || self.birthday_month.is_some()
            || self.birthday_day.is_some()
    }
}

fn parse_query(value: Option<String>) -> Result<Option<TextQuery>, AppError> {
    value
        .as_deref()
        .map(TextQuery::parse)
        .transpose()
        .map_err(|error| AppError::BadRequest(error.to_string()))
        .map(|query| query.filter(|query| !query.is_empty()))
}

fn validate_birthday(month: Option<i16>, day: Option<i16>) -> Result<(), AppError> {
    if let Some(month) = month
        && !(1..=12).contains(&month)
    {
        return Err(AppError::BadRequest(
            "birthday_month must be between 1 and 12".to_owned(),
        ));
    }
    if let Some(day) = day
        && !(1..=31).contains(&day)
    {
        return Err(AppError::BadRequest(
            "birthday_day must be between 1 and 31".to_owned(),
        ));
    }
    Ok(())
}

fn parse_i16(field: &str, value: &str) -> Result<i16, AppError> {
    value
        .parse()
        .map_err(|_| AppError::BadRequest(format!("{field} must be an integer")))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn first_present<'a>(fields: &[(&'a str, bool)]) -> Option<&'a str> {
    fields
        .iter()
        .find_map(|(name, present)| present.then_some(*name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_query() -> GameDataListQuery {
        GameDataListQuery {
            q: None,
            element: None,
            weapon_type: None,
            rarity: None,
            region: None,
            affiliation: None,
            cv: None,
            path: None,
            combat_type: None,
            camp: None,
            specialty_id: None,
            specialty: None,
            element_id: None,
            hit_type_id: None,
            hit_type: None,
            camp_id: None,
            gender: None,
            special_element: None,
            special: None,
            birthday_month: None,
            birthday_day: None,
            limit: None,
            offset: None,
        }
    }

    #[test]
    fn creates_game_specific_character_filters() {
        let mut query = empty_query();
        query.element = Some("Pyro".to_owned());
        let filter = query
            .into_filter("ys".to_owned(), "character".to_owned())
            .expect("ys character filter should be valid");
        assert!(matches!(
            filter.collection_filter,
            Some(GameDataCollectionFilter::YsCharacter(_))
        ));
    }

    #[test]
    fn rejects_character_filters_for_other_collections_and_games() {
        let mut weapon = empty_query();
        weapon.element = Some("Pyro".to_owned());
        assert!(matches!(
            weapon.into_filter("ys".to_owned(), "weapon".to_owned()),
            Err(AppError::BadRequest(_))
        ));

        let mut sr = empty_query();
        sr.gender = Some("female".to_owned());
        assert!(matches!(
            sr.into_filter("sr".to_owned(), "character".to_owned()),
            Err(AppError::BadRequest(_))
        ));
    }
}
