use uuid::Uuid;

use crate::{ApplicationError, ApplicationRepository, ApplicationServices};

/// OAuth 登录时接受的 GitHub 身份资料
#[derive(Debug, Clone)]
pub struct GithubUserProfile {
    pub provider_user_id: String,
    pub provider_login: String,
    pub display_name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
}

/// 与 refresh token 一并记录的请求元数据
#[derive(Debug, Clone)]
pub struct RefreshTokenMetadata {
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

/// 签发 access token 所需的最小用户资料
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
}

/// 当前用户用例返回的已认证用户资料
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub groups: Vec<String>,
    pub is_admin: bool,
}

impl<R> ApplicationServices<R>
where
    R: ApplicationRepository,
{
    /// 创建或更新 GitHub 资料对应的本地用户
    pub async fn upsert_github_user(
        &self,
        profile: GithubUserProfile,
    ) -> Result<AuthUser, ApplicationError> {
        Ok(self.repository.upsert_github_user(profile).await?)
    }

    /// 为已认证用户保存 refresh token 哈希
    pub async fn save_refresh_token(
        &self,
        user_id: Uuid,
        refresh_token_hash: String,
        metadata: RefreshTokenMetadata,
    ) -> Result<(), ApplicationError> {
        self.repository
            .save_refresh_token(user_id, refresh_token_hash, metadata)
            .await?;
        Ok(())
    }

    /// 轮换 refresh token 哈希并返回对应用户
    pub async fn rotate_refresh_token(
        &self,
        old_refresh_token_hash: String,
        new_refresh_token_hash: String,
        metadata: RefreshTokenMetadata,
    ) -> Result<AuthUser, ApplicationError> {
        Ok(self
            .repository
            .rotate_refresh_token(old_refresh_token_hash, new_refresh_token_hash, metadata)
            .await?)
    }

    /// 吊销存在的 refresh token 哈希
    pub async fn revoke_refresh_token(
        &self,
        refresh_token_hash: String,
    ) -> Result<(), ApplicationError> {
        self.repository
            .revoke_refresh_token(refresh_token_hash)
            .await?;
        Ok(())
    }

    /// 查找 access token subject 对应的启用用户
    pub async fn find_current_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<CurrentUser>, ApplicationError> {
        Ok(self.repository.find_current_user(user_id).await?)
    }
}
