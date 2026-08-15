use akasha_application::auth::{AuthUser, GithubUserProfile, RefreshTokenMetadata};
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

/// 创建或更新本地用户、OAuth 账号及所需默认用户组
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

/// 保存 refresh token 哈希、过期时间及请求元数据
pub async fn save_refresh_token(
    db: &Db,
    user_id: Uuid,
    refresh_token_hash: String,
    metadata: RefreshTokenMetadata,
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
        user_agent: Set(metadata.user_agent),
        ip_address: Set(metadata.ip_address),
    }
    .insert(db.conn())
    .await
    .map_err(DbError::Query)
}

/// 在一个事务中校验并轮换 refresh token
pub async fn rotate_refresh_token(
    db: &Db,
    old_refresh_token_hash: String,
    new_refresh_token_hash: String,
    metadata: RefreshTokenMetadata,
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

                // SQLite 不支持行级排他锁，使用带未吊销条件的更新保证轮换只成功一次
                let update = user_refresh_tokens::Entity::update_many()
                    .set(user_refresh_tokens::ActiveModel {
                        revoked_at: Set(Some(now)),
                        last_used_at: Set(Some(now)),
                        replaced_by_token_id: Set(Some(new_token_id)),
                        ..Default::default()
                    })
                    .filter(user_refresh_tokens::Column::Id.eq(old_token.id))
                    .filter(user_refresh_tokens::Column::RevokedAt.is_null())
                    .exec(txn)
                    .await?;
                if update.rows_affected != 1 {
                    return Err(DbErr::Custom("refresh token already rotated".to_string()));
                }

                user_refresh_tokens::ActiveModel {
                    id: Set(new_token_id),
                    user_id: Set(user.id),
                    token_hash: Set(new_refresh_token_hash),
                    expires_at: Set(now + Duration::days(30)),
                    revoked_at: Set(None),
                    replaced_by_token_id: Set(None),
                    created_at: Set(now),
                    last_used_at: Set(None),
                    user_agent: Set(metadata.user_agent),
                    ip_address: Set(metadata.ip_address),
                }
                .insert(txn)
                .await?;

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

/// 在 refresh token 尚未被吊销时将其吊销
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

/// 当用户尚不在该组时创建用户组记录
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
    }
    .insert(db)
    .await?;
    Ok(())
}

/// 将 SeaORM 事务失败归一化为数据库错误类型
fn map_transaction_error(error: TransactionError<DbErr>) -> DbError {
    match error {
        TransactionError::Connection(error) | TransactionError::Transaction(error) => {
            DbError::Query(error)
        }
    }
}
