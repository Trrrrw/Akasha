use crate::{ApplicationError, ApplicationRepository, ApplicationServices};
use serde::Deserialize;

/// Ys 角色目录；后续字段在本游戏模型内扩展
#[derive(Debug, Clone, Default)]
pub struct YsCharacterListFilter {
    pub birthday_only: bool,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct YsCharacter {
    pub id: String,
    pub name: String,
    pub icon_url: Option<String>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
}

/// Sr 角色目录；后续字段在本游戏模型内扩展
#[derive(Debug, Clone, Default)]
pub struct SrCharacterListFilter {
    pub birthday_only: bool,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SrCharacter {
    pub id: String,
    pub name: String,
    pub icon_url: Option<String>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
}

/// Zzz 角色目录；后续字段在本游戏模型内扩展
#[derive(Debug, Clone, Default)]
pub struct ZzzCharacterListFilter {
    pub birthday_only: bool,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZzzCharacter {
    pub id: String,
    pub name: String,
    pub icon_url: Option<String>,
    pub birthday_month: Option<i16>,
    pub birthday_day: Option<i16>,
}

impl<R: ApplicationRepository> ApplicationServices<R> {
    /// 读取 Ys 角色目录
    pub async fn list_ys_characters(
        &self,
        filter: YsCharacterListFilter,
    ) -> Result<(u64, Vec<YsCharacter>), ApplicationError> {
        Ok(self.repository.list_ys_characters(filter).await?)
    }
    /// 读取 Sr 角色目录
    pub async fn list_sr_characters(
        &self,
        filter: SrCharacterListFilter,
    ) -> Result<(u64, Vec<SrCharacter>), ApplicationError> {
        Ok(self.repository.list_sr_characters(filter).await?)
    }
    /// 读取 Zzz 角色目录
    pub async fn list_zzz_characters(
        &self,
        filter: ZzzCharacterListFilter,
    ) -> Result<(u64, Vec<ZzzCharacter>), ApplicationError> {
        Ok(self.repository.list_zzz_characters(filter).await?)
    }
}
