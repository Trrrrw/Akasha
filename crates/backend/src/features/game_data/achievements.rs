use super::endpoints::validate_game;
use crate::{
    http::{
        error::AppError,
        path::{GamePath, require_game_data_collection},
        response::{ErrorResponse, ListResponse, PageResponse},
    },
    state::AppState,
};
use akasha_application::{
    achievements::{Achievement, AchievementListFilter},
    search::TextQuery,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct AchievementQuery {
    /// 按名称和描述搜索
    q: Option<String>,
    /// 成就分类 ID
    group_id: Option<String>,
    /// 筛选隐藏成就；未知状态不匹配 true 或 false
    hidden: Option<bool>,
    /// 每页数量，1 到 100
    limit: Option<u64>,
    /// 分页偏移
    offset: Option<u64>,
}

/// 一个成就的公共定义，玩家状态由客户端保存
#[derive(Serialize, ToSchema)]
pub(super) struct AchievementResponse {
    id: String,
    name: String,
    description: String,
    group_id: String,
    group_name: String,
    group_order: i64,
    order: i64,
    hidden: Option<bool>,
    rewards: Vec<AchievementRewardResponse>,
    target: Option<u64>,
    previous_id: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub(super) struct AchievementRewardResponse {
    item_id: String,
    name: Option<String>,
    count: u64,
}
#[derive(Serialize, ToSchema)]
pub(super) struct AchievementGroupResponse {
    id: String,
    name: String,
    order: i64,
    total: u64,
}

impl From<Achievement> for AchievementResponse {
    fn from(a: Achievement) -> Self {
        Self {
            id: a.id,
            name: a.name,
            description: a.description,
            group_id: a.group_id,
            group_name: a.group_name,
            group_order: a.group_order,
            order: a.order,
            hidden: a.hidden,
            rewards: a
                .rewards
                .into_iter()
                .map(|r| AchievementRewardResponse {
                    item_id: r.item_id,
                    name: r.name,
                    count: r.count,
                })
                .collect(),
            target: a.target,
            previous_id: a.previous_id,
        }
    }
}

#[utoipa::path(get, path="/games/{game_id}/achievements", tag="Game Data", summary="获取成就目录", description="从数据库分页读取规范化成就定义，不包含玩家完成状态", params(GamePath,AchievementQuery), responses((status=200,body=PageResponse<AchievementResponse>),(status=400,body=ErrorResponse),(status=404,body=ErrorResponse),(status=500,body=ErrorResponse)))]
pub(super) async fn list(
    Path(GamePath { game_id }): Path<GamePath>,
    State(state): State<AppState>,
    Query(query): Query<AchievementQuery>,
) -> Result<Json<PageResponse<AchievementResponse>>, AppError> {
    validate_game(&game_id)?;
    require_game_data_collection(&state, &game_id, "achievement").await?;
    let limit = query.limit.unwrap_or(100);
    if !(1..=100).contains(&limit)
        || query
            .group_id
            .as_ref()
            .is_some_and(|id| id.trim().is_empty() || id.len() > 128)
    {
        return Err(AppError::BadRequest("invalid achievement query".to_owned()));
    }
    let offset = query.offset.unwrap_or(0);
    let text = query
        .q
        .as_deref()
        .map(TextQuery::parse)
        .transpose()
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    let (total, items) = state
        .application()
        .list_achievements(
            game_id,
            text,
            AchievementListFilter {
                group_id: query.group_id,
                hidden: query.hidden,
            },
            limit,
            offset,
        )
        .await?;
    Ok(Json(PageResponse {
        total,
        limit,
        offset,
        items: items.into_iter().map(Into::into).collect(),
        meta: (),
    }))
}

#[utoipa::path(get, path="/games/{game_id}/achievement-groups", tag="Game Data", summary="获取成就分类", description="聚合已保存的成就分类和数量", params(GamePath), responses((status=200,body=ListResponse<AchievementGroupResponse>),(status=404,body=ErrorResponse),(status=500,body=ErrorResponse)))]
pub(super) async fn groups(
    Path(GamePath { game_id }): Path<GamePath>,
    State(state): State<AppState>,
) -> Result<Json<ListResponse<AchievementGroupResponse>>, AppError> {
    validate_game(&game_id)?;
    require_game_data_collection(&state, &game_id, "achievement").await?;
    let items: Vec<_> = state
        .application()
        .list_achievement_groups(&game_id)
        .await?
        .into_iter()
        .map(|g| AchievementGroupResponse {
            id: g.id,
            name: g.name,
            order: g.order,
            total: g.total,
        })
        .collect();
    Ok(Json(ListResponse {
        total: items.len() as u64,
        items,
    }))
}
