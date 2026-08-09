use serde::Deserialize;
use utoipa::IntoParams;

use akasha_application::characters::CharacterListFilter;

use crate::http::error::AppError;

const DEFAULT_PAGE_LIMIT: u64 = 20;
const MAX_PAGE_LIMIT: u64 = 100;

/// 角色列表接口接受的查询参数
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct CharacterListQuery {
    /// 角色名称关键词
    pub q: Option<String>,
    /// 性别，仅支持 male 或 female
    pub gender: Option<String>,
    /// 配音演员关键词
    pub cv: Option<String>,
    /// 生日月份，取值 1 到 12
    pub birthday_month: Option<i16>,
    /// 每页数量，默认 20，最大 100
    pub limit: Option<u64>,
    /// 分页偏移，默认 0
    pub offset: Option<u64>,
}

impl CharacterListQuery {
    /// 将已校验的 HTTP 查询参数转换为应用层列表筛选条件
    pub(super) fn into_filter(self, game_id: String) -> Result<CharacterListFilter, AppError> {
        let gender = self
            .gender
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if !matches!(gender.as_deref(), None | Some("male" | "female")) {
            return Err(AppError::BadRequest(
                "gender must be male or female".to_owned(),
            ));
        }
        if let Some(month) = self.birthday_month
            && !(1..=12).contains(&month)
        {
            return Err(AppError::BadRequest(
                "birthday_month must be between 1 and 12".to_owned(),
            ));
        }

        Ok(CharacterListFilter {
            game_id,
            query: self.q,
            gender,
            voice_actor: self.cv,
            birthday_month: self.birthday_month,
            limit: self
                .limit
                .unwrap_or(DEFAULT_PAGE_LIMIT)
                .clamp(1, MAX_PAGE_LIMIT),
            offset: self.offset.unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::http::error::AppError;

    use super::CharacterListQuery;

    /// 创建仅覆盖目标筛选值的查询对象
    fn query(gender: Option<&str>, birthday_month: Option<i16>) -> CharacterListQuery {
        CharacterListQuery {
            q: None,
            gender: gender.map(ToOwned::to_owned),
            cv: None,
            birthday_month,
            limit: None,
            offset: None,
        }
    }

    /// 规范化合法性别并拒绝未知值
    #[test]
    fn validates_gender() {
        let filter = query(Some(" female "), None)
            .into_filter("ys".to_owned())
            .expect("应接受合法性别");

        assert_eq!(filter.gender.as_deref(), Some("female"));
        assert!(matches!(
            query(Some("unknown"), None).into_filter("ys".to_owned()),
            Err(AppError::BadRequest(_))
        ));
    }

    /// 生日月份必须位于自然月份范围内
    #[test]
    fn validates_birthday_month() {
        assert!(matches!(
            query(None, Some(13)).into_filter("ys".to_owned()),
            Err(AppError::BadRequest(_))
        ));
    }
}
