use serde::Deserialize;

use crate::{ApplicationError, ApplicationRepository, ApplicationServices, search::TextQuery};

/// 原神角色列表筛选条件
#[derive(Debug, Clone)]
pub struct YsCharacterListFilter {
    pub query: Option<TextQuery>,
    pub element: Option<String>,
    pub weapon_type: Option<String>,
    pub rarity: Option<String>,
    pub region: Option<String>,
    pub affiliation: Option<String>,
    pub voice_actor: Option<String>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub special: Option<bool>,
    pub birthday_only: bool,
    pub limit: u64,
    pub offset: u64,
}

/// 星铁角色列表筛选条件
#[derive(Debug, Clone)]
pub struct SrCharacterListFilter {
    pub query: Option<TextQuery>,
    pub path: Option<String>,
    pub combat_type: Option<String>,
    pub rarity: Option<String>,
    pub camp: Option<String>,
    pub voice_actor: Option<String>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub birthday_only: bool,
    pub limit: u64,
    pub offset: u64,
}

/// 绝区零角色列表筛选条件
#[derive(Debug, Clone)]
pub struct ZzzCharacterListFilter {
    pub query: Option<TextQuery>,
    pub specialty_id: Option<i32>,
    pub specialty: Option<String>,
    pub element_id: Option<i32>,
    pub element: Option<String>,
    pub hit_type_id: Option<i32>,
    pub hit_type: Option<String>,
    pub camp_id: Option<i32>,
    pub camp: Option<String>,
    pub rarity: Option<i16>,
    pub gender: Option<String>,
    pub special_element: Option<String>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub birthday_only: bool,
    pub limit: u64,
    pub offset: u64,
}

/// 原神角色公开读取模型
#[derive(Debug, Clone, Deserialize)]
pub struct YsCharacter {
    pub id: String,
    pub name: String,
    pub name_en: String,
    pub name_ja: String,
    pub name_ko: String,
    pub description: String,
    pub description_en: String,
    pub icon_url: String,
    pub release_date: Option<String>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub rarity: Option<String>,
    pub weapon_type: Option<String>,
    pub element: Option<String>,
    pub constellation: Option<String>,
    pub region: Option<String>,
    pub affiliation: Option<String>,
    pub title: Option<String>,
    pub cv_zh: Option<String>,
    pub cv_en: Option<String>,
    pub cv_ja: Option<String>,
    pub cv_ko: Option<String>,
    pub base_hp: Option<f64>,
    pub base_atk: Option<f64>,
    pub base_def: Option<f64>,
    pub crit_rate: Option<f64>,
    pub crit_dmg: Option<f64>,
    pub elemental_mastery: Option<f64>,
    pub stamina_recovery: Option<f64>,
    pub special: bool,
}

/// 星铁角色公开读取模型
#[derive(Debug, Clone, Deserialize)]
pub struct SrCharacter {
    pub id: String,
    pub name: String,
    pub name_en: String,
    pub name_ja: String,
    pub name_ko: String,
    pub description: String,
    pub description_en: String,
    pub icon_url: String,
    pub release_at: Option<i64>,
    pub rarity: String,
    pub path: String,
    pub combat_type: String,
    pub camp: Option<String>,
    pub cv_zh: Option<String>,
    pub cv_en: Option<String>,
    pub cv_ja: Option<String>,
    pub cv_ko: Option<String>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub avatar_vo_tag: String,
    pub sp_need: Option<i16>,
}

/// 绝区零角色公开读取模型
#[derive(Debug, Clone, Deserialize)]
pub struct ZzzCharacter {
    pub id: String,
    pub name: String,
    pub name_en: String,
    pub name_ja: String,
    pub name_ko: String,
    pub description: Option<String>,
    pub description_en: String,
    pub icon_url: String,
    pub code_name: String,
    pub rarity: i16,
    pub specialty_id: i32,
    pub specialty: String,
    pub element_id: i32,
    pub element: String,
    pub special_element: Option<String>,
    pub special_element_title: Option<String>,
    pub special_element_description: Option<String>,
    pub special_element_icon: Option<String>,
    pub hit_type_id: i32,
    pub hit_type: String,
    pub camp_id: i32,
    pub camp: String,
    pub gender: String,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
    pub full_name: Option<String>,
    pub stature: Option<String>,
    pub live2d: Option<String>,
}

impl<R> ApplicationServices<R>
where
    R: ApplicationRepository,
{
    /// 列出原神角色
    pub async fn list_ys_characters(
        &self,
        filter: YsCharacterListFilter,
    ) -> Result<(u64, Vec<YsCharacter>), ApplicationError> {
        Ok(self.repository.list_ys_characters(filter).await?)
    }

    /// 列出星铁角色
    pub async fn list_sr_characters(
        &self,
        filter: SrCharacterListFilter,
    ) -> Result<(u64, Vec<SrCharacter>), ApplicationError> {
        Ok(self.repository.list_sr_characters(filter).await?)
    }

    /// 列出绝区零角色
    pub async fn list_zzz_characters(
        &self,
        filter: ZzzCharacterListFilter,
    ) -> Result<(u64, Vec<ZzzCharacter>), ApplicationError> {
        Ok(self.repository.list_zzz_characters(filter).await?)
    }
}
