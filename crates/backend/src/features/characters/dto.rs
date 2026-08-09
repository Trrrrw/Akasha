use akasha_application::characters::CharacterSummary;
use serde::Serialize;
use utoipa::ToSchema;

/// 公开角色接口返回的角色资料
#[derive(Serialize, ToSchema)]
#[schema(description = "角色信息")]
pub(super) struct CharacterResponse {
    /// 角色记录 ID
    id: String,
    /// 游戏内物品 ID
    item_id: String,
    /// 角色名称
    name: String,
    /// 角色简介
    description: Option<String>,
    /// 性别标识
    gender: Option<String>,
    /// 生日月份
    birthday_month: Option<i16>,
    /// 生日日期
    birthday_day: Option<i16>,
    /// 中文配音演员
    cv: Option<String>,
}

impl From<CharacterSummary> for CharacterResponse {
    /// 将应用层角色读取模型转换为公开响应
    fn from(value: CharacterSummary) -> Self {
        Self {
            id: value.id,
            item_id: value.item_id,
            name: value.name,
            description: value.description,
            gender: value.gender,
            birthday_month: value.birthday_month,
            birthday_day: value.birthday_day,
            cv: value.voice_actor,
        }
    }
}
