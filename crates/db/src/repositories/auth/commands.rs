use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbErr, EntityTrait, QueryFilter,
    TransactionError, TransactionTrait,
};
use uuid::Uuid;

use crate::{
    Db, DbError,
    entities::{oauth_accounts, user_groups, user_refresh_tokens, users},
    models::UserGroup,
};

use super::projections::AuthUser;

pub struct GithubUserProfile {
    pub provider_user_id: String,
    pub provider_login: String,
    pub display_name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
}

/// 根据 OAuth 用户资料创建或更新本地用户、OAuth 账号和默认用户组
pub async fn upsert_github_user(db: &Db, profile: GithubUserProfile) -> Result<AuthUser, DbError> {
    let is_admin = profile.is_admin;

    db.conn()
        .transaction::<_, users::Model, DbErr>(|txn| {
            Box::pin(async move {
                let now = Utc::now().fixed_offset();
                let account = oauth_accounts::Entity::find_by_id((
                    "github".to_string(),
                    profile.provider_user_id.clone(),
                ))
                .one(txn)
                .await?;

                if let Some(account) = account {
                    let user = users::Entity::find_by_id(account.user_id)
                        .one(txn)
                        .await?
                        .ok_or_else(|| DbErr::Custom("oauth account user not found".to_string()))?;
                    if user.disabled_at.is_some() {
                        return Err(DbErr::Custom("user disabled".to_string()));
                    }

                    let user_id = user.id;
                    let mut account_active: oauth_accounts::ActiveModel = account.into();
                    account_active.provider_login = Set(profile.provider_login);
                    account_active.email = Set(profile.email);
                    account_active.avatar_url = Set(profile.avatar_url.clone());
                    account_active.updated_at = Set(now);
                    account_active.update(txn).await?;

                    let mut user_active: users::ActiveModel = user.into();
                    user_active.display_name = Set(profile.display_name);
                    user_active.avatar_url = Set(profile.avatar_url);
                    user_active.updated_at = Set(now);
                    let user = user_active.update(txn).await?;

                    if profile.is_admin {
                        ensure_user_group(txn, user_id, UserGroup::Admin, now).await?;
                    }
                    return Ok(user);
                }

                let user_id = Uuid::new_v4();
                let user = users::ActiveModel {
                    id: Set(user_id),
                    display_name: Set(profile.display_name),
                    avatar_url: Set(profile.avatar_url.clone()),
                    disabled_at: Set(None),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(txn)
                .await?;

                oauth_accounts::ActiveModel {
                    provider: Set("github".to_string()),
                    provider_user_id: Set(profile.provider_user_id),
                    user_id: Set(user_id),
                    provider_login: Set(profile.provider_login),
                    email: Set(profile.email),
                    avatar_url: Set(profile.avatar_url),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(txn)
                .await?;

                ensure_user_group(txn, user_id, UserGroup::User, now).await?;
                if profile.is_admin {
                    ensure_user_group(txn, user_id, UserGroup::Admin, now).await?;
                }
                Ok(user)
            })
        })
        .await
        .map_err(map_transaction_error)
        .map(|user| AuthUser {
            id: user.id,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            is_admin,
        })
}

pub struct RefreshTokenMeta {
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

/// 保存 refresh token 的哈希、过期时间和请求来源信息
pub async fn save_refresh_token(
    db: &Db,
    user_id: Uuid,
    refresh_token_hash: String,
    meta: RefreshTokenMeta,
) -> Result<user_refresh_tokens::Model, DbError> {
    let now = Utc::now().fixed_offset();
    user_refresh_tokens::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        token_hash: Set(refresh_token_hash),
        expires_at: Set(now + Duration::days(30)),
        revoked_at: Set(None),
        replaced_by_token_id: Set(None),
        created_at: Set(now),
        last_used_at: Set(None),
        user_agent: Set(meta.user_agent),
        ip_address: Set(meta.ip_address),
        ..Default::default()
    }
    .insert(db.conn())
    .await
    .map_err(DbError::Query)
}

/// 校验并轮换 refresh token
pub async fn rotate_refresh_token(
    db: &Db,
    old_refresh_token_hash: String,
    new_refresh_token_hash: String,
    meta: RefreshTokenMeta,
) -> Result<AuthUser, DbError> {
    db.conn()
        .transaction::<_, AuthUser, DbErr>(|txn| {
            Box::pin(async move {
                let now = Utc::now().fixed_offset();
                let old_token = user_refresh_tokens::Entity::find()
                    .filter(user_refresh_tokens::Column::TokenHash.eq(old_refresh_token_hash))
                    .one(txn)
                    .await?
                    .ok_or_else(|| DbErr::Custom("refresh token not found".to_string()))?;
                if old_token.revoked_at.is_some() {
                    return Err(DbErr::Custom("refresh token revoked".to_string()));
                }
                if old_token.expires_at <= now {
                    return Err(DbErr::Custom("refresh token expired".to_string()));
                }

                let user = users::Entity::find_by_id(old_token.user_id)
                    .one(txn)
                    .await?
                    .ok_or_else(|| DbErr::Custom("refresh token user not found".to_string()))?;
                if user.disabled_at.is_some() {
                    return Err(DbErr::Custom("user disabled".to_string()));
                }

                let is_admin = user_groups::Entity::find_by_id((user.id, UserGroup::Admin))
                    .one(txn)
                    .await?
                    .is_some();
                let new_token_id = Uuid::new_v4();
                user_refresh_tokens::ActiveModel {
                    id: Set(new_token_id),
                    user_id: Set(user.id),
                    token_hash: Set(new_refresh_token_hash),
                    expires_at: Set(now + Duration::days(30)),
                    revoked_at: Set(None),
                    replaced_by_token_id: Set(None),
                    created_at: Set(now),
                    last_used_at: Set(None),
                    user_agent: Set(meta.user_agent),
                    ip_address: Set(meta.ip_address),
                    ..Default::default()
                }
                .insert(txn)
                .await?;

                let mut old_token_active: user_refresh_tokens::ActiveModel = old_token.into();
                old_token_active.revoked_at = Set(Some(now));
                old_token_active.last_used_at = Set(Some(now));
                old_token_active.replaced_by_token_id = Set(Some(new_token_id));
                old_token_active.update(txn).await?;

                Ok(AuthUser {
                    id: user.id,
                    display_name: user.display_name,
                    avatar_url: user.avatar_url,
                    is_admin,
                })
            })
        })
        .await
        .map_err(map_transaction_error)
}

/// 吊销 refresh token
pub async fn revoke_refresh_token(db: &Db, refresh_token_hash: String) -> Result<(), DbError> {
    let now = Utc::now().fixed_offset();
    let token = user_refresh_tokens::Entity::find()
        .filter(user_refresh_tokens::Column::TokenHash.eq(refresh_token_hash))
        .one(db.conn())
        .await
        .map_err(DbError::Query)?
        .ok_or_else(|| DbError::Query(DbErr::Custom("refresh token not found".to_string())))?;
    if token.revoked_at.is_some() {
        return Ok(());
    }
    let mut active: user_refresh_tokens::ActiveModel = token.into();
    active.revoked_at = Set(Some(now));
    active.last_used_at = Set(Some(now));
    active.update(db.conn()).await.map_err(DbError::Query)?;
    Ok(())
}

async fn ensure_user_group<C>(
    db: &C,
    user_id: Uuid,
    group: UserGroup,
    created_at: chrono::DateTime<chrono::FixedOffset>,
) -> Result<(), DbErr>
where
    C: sea_orm::ConnectionTrait,
{
    if user_groups::Entity::find_by_id((user_id, group.clone()))
        .one(db)
        .await?
        .is_some()
    {
        return Ok(());
    }
    user_groups::ActiveModel {
        user_id: Set(user_id),
        group: Set(group),
        created_at: Set(created_at),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

fn map_transaction_error(error: TransactionError<DbErr>) -> DbError {
    match error {
        TransactionError::Connection(error) | TransactionError::Transaction(error) => {
            DbError::Query(error)
        }
    }
}
