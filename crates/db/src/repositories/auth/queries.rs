use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::{
    Db, DbError,
    entities::{user_groups, users},
    models::UserGroup,
};

use super::projections::CurrentUser;

/// 查询当前用户资料和用户组
pub async fn find_current_user(db: &Db, user_id: Uuid) -> Result<Option<CurrentUser>, DbError> {
    let Some(user) = users::Entity::find_by_id(user_id)
        .one(db.conn())
        .await
        .map_err(DbError::Query)?
    else {
        return Ok(None);
    };

    if user.disabled_at.is_some() {
        return Ok(None);
    }

    let group_rows = user_groups::Entity::find()
        .filter(user_groups::Column::UserId.eq(user_id))
        .all(db.conn())
        .await
        .map_err(DbError::Query)?;

    let is_admin = group_rows
        .iter()
        .any(|row| matches!(row.group, UserGroup::Admin));
    let groups = group_rows
        .into_iter()
        .map(|row| match row.group {
            UserGroup::Admin => "admin".to_string(),
            UserGroup::User => "user".to_string(),
        })
        .collect();

    Ok(Some(CurrentUser {
        id: user.id,
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        groups,
        is_admin,
    }))
}
