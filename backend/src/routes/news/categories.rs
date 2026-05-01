use axum::{Json, extract::Query, http::StatusCode};
use db::entities::{games, news_categories, news_categories_link, news_items};
use sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use axum_mcp::{MCPInputSchema, mcp};

#[mcp]
#[utoipa::path(
    get,
    path = "/categories",
    tag = "News",
    summary = "获取指定游戏的新闻分类列表",
    description = "根据新闻来源和游戏代码返回该来源在指定游戏下的新闻自动分类、关联新闻数量和最新新闻封面；传入 is_video=true 时只返回有视频新闻的分类。",
    params(NewsCategoriesQuery),
    responses(
        (status = 200, body = NewsCategoriesResponse),
        (status = 500, body = NewsCategoryErrorResponse)
    )
)]
pub async fn news_categories(
    Query(query): Query<NewsCategoriesQuery>,
) -> Result<Json<NewsCategoriesResponse>, (StatusCode, Json<NewsCategoryErrorResponse>)> {
    let Some(game_code) = games::Entity::resolve_game_code(&query.game)
        .await
        .map_err(internal_error)?
    else {
        return Ok(Json(NewsCategoriesResponse {
            source: query.source,
            game_code: query.game,
            categories: Vec::new(),
        }));
    };

    let categories = news_categories::Entity::find()
        .filter(news_categories::Column::GameCode.eq(&game_code))
        .order_by(news_categories::Column::Index, Order::Asc)
        .all(db::pool())
        .await
        .map_err(internal_error)?;

    let mut data = Vec::new();

    for category in categories {
        let Some(summary) =
            NewsCategorySummary::from_model(category, &query.source, query.is_video)
                .await
                .map_err(internal_error)?
        else {
            continue;
        };

        data.push(summary);
    }

    Ok(Json(NewsCategoriesResponse {
        source: query.source,
        game_code,
        categories: data,
    }))
}

fn internal_error(err: sea_orm::DbErr) -> (StatusCode, Json<NewsCategoryErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(NewsCategoryErrorResponse {
            message: err.to_string(),
        }),
    )
}

#[derive(Deserialize, IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Query)]
pub struct NewsCategoriesQuery {
    /// 新闻来源，例如 official_site。
    source: String,
    /// 游戏代码或中英文名称，例如 hk4e、原神、Genshin Impact。
    game: String,
    /// 按新闻视频类型筛选分类；true 表示只统计视频新闻，false 表示只统计非视频新闻。
    is_video: Option<bool>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻自动分类基础信息。")]
pub struct NewsCategorySummary {
    /// 新闻自动分类名称。
    title: String,
    /// 分类展示排序。
    index: i32,
    /// 当前分类关联的新闻数量；传入 is_video 时按该视频类型计数。
    news_count: u64,
    /// 当前分类下最新新闻的封面图片地址；传入 is_video 时取该视频类型下的最新新闻封面。
    cover: String,
}

impl NewsCategorySummary {
    async fn from_model(
        category: news_categories::Model,
        source: &str,
        is_video: Option<bool>,
    ) -> Result<Option<Self>, sea_orm::DbErr> {
        let links = news_categories_link::Entity::find()
            .filter(news_categories_link::Column::CategoryTitle.eq(&category.title))
            .filter(news_categories_link::Column::CategoryGameBelong.eq(&category.game_code))
            .filter(news_categories_link::Column::NewsSourceBelong.eq(source))
            .all(db::pool())
            .await?;

        let mut news_count = 0_u64;
        let mut latest_news = None::<news_items::Model>;

        for link in links {
            let Some(news) = news_items::Entity::find_by_id((
                link.news_remote_id,
                link.news_game_belong,
                link.news_source_belong,
            ))
            .one(db::pool())
            .await?
            else {
                continue;
            };

            if let Some(expected) = is_video
                && news.is_video != expected
            {
                continue;
            }

            news_count += 1;

            if latest_news
                .as_ref()
                .is_none_or(|latest| news.publish_time > latest.publish_time)
            {
                latest_news = Some(news);
            }
        }

        if is_video.is_some() && news_count == 0 {
            return Ok(None);
        }

        Ok(Some(Self {
            title: category.title,
            index: category.index,
            news_count,
            cover: latest_news.map(|news| news.cover).unwrap_or_default(),
        }))
    }
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻自动分类列表响应。")]
pub struct NewsCategoriesResponse {
    /// 新闻来源。
    source: String,
    /// 游戏代码。
    game_code: String,
    /// 新闻自动分类列表。
    categories: Vec<NewsCategorySummary>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻自动分类接口错误响应。")]
pub struct NewsCategoryErrorResponse {
    /// 错误信息。
    message: String,
}
