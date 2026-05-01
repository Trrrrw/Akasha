use axum::{Json, extract::Query, http::StatusCode};
use chrono::{DateTime, FixedOffset};
use db::entities::{games, news_items, news_sources};
use sea_orm::{ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use axum_mcp::{MCPInputSchema, mcp};

#[mcp]
#[utoipa::path(
    get,
    path = "/sources",
    tag = "News",
    summary = "获取新闻来源列表",
    description = "返回当前数据库中已收录的新闻来源、来源说明、新闻数量、最新发布时间，以及每个来源在各游戏下的数据覆盖情况。可通过 game 参数获取指定游戏下可用的新闻来源。",
    params(NewsSourcesQuery),
    responses(
        (status = 200, body = NewsSourcesResponse),
        (status = 500, body = NewsSourceErrorResponse)
    )
)]
pub async fn news_sources(
    Query(query): Query<NewsSourcesQuery>,
) -> Result<Json<NewsSourcesResponse>, (StatusCode, Json<NewsSourceErrorResponse>)> {
    let game_code = match query.game.as_deref() {
        Some(game) => match games::Entity::resolve_game_code(game)
            .await
            .map_err(internal_error)?
        {
            Some(game_code) => Some(game_code),
            None => {
                return Ok(Json(NewsSourcesResponse {
                    sources: Vec::new(),
                }));
            }
        },
        None => None,
    };
    let sources = news_sources::Entity::find()
        .all(db::pool())
        .await
        .map_err(internal_error)?;

    let mut data = Vec::new();

    for source in sources {
        let summary = NewsSourceSummary::from_model(source, game_code.as_deref())
            .await
            .map_err(internal_error)?;

        if game_code.is_some() && summary.news_count == 0 {
            continue;
        }

        data.push(summary);
    }

    Ok(Json(NewsSourcesResponse { sources: data }))
}

fn internal_error(err: sea_orm::DbErr) -> (StatusCode, Json<NewsSourceErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(NewsSourceErrorResponse {
            message: err.to_string(),
        }),
    )
}

#[derive(Deserialize, ToSchema, utoipa::IntoParams, MCPInputSchema)]
#[into_params(parameter_in = Query)]
#[schema(description = "新闻来源列表查询参数。")]
pub struct NewsSourcesQuery {
    /// 游戏代码或中英文名称。传入后只返回该游戏下有新闻数据的来源。
    game: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻来源基础信息和数据覆盖情况。")]
pub struct NewsSourceSummary {
    /// 新闻来源 ID，例如 official_site。
    source_id: String,
    /// 新闻来源展示名称。
    display_name: String,
    /// 新闻来源说明。
    description: String,
    /// 当前来源关联的新闻数量。
    news_count: u64,
    /// 当前来源下最新新闻的发布时间，RFC3339 格式。
    latest_published_at: Option<String>,
    /// 当前来源按游戏统计的新闻数量。
    games: Vec<NewsSourceGameSummary>,
}

impl NewsSourceSummary {
    async fn from_model(
        source: news_sources::Model,
        game: Option<&str>,
    ) -> Result<Self, sea_orm::DbErr> {
        let mut news_count_query =
            news_items::Entity::find().filter(news_items::Column::Source.eq(&source.name));
        if let Some(game) = game {
            news_count_query = news_count_query.filter(news_items::Column::GameCode.eq(game));
        }
        let news_count = news_count_query.count(db::pool()).await?;

        let mut latest_query =
            news_items::Entity::find().filter(news_items::Column::Source.eq(&source.name));
        if let Some(game) = game {
            latest_query = latest_query.filter(news_items::Column::GameCode.eq(game));
        }
        let latest_published_at = latest_query
            .order_by(news_items::Column::PublishTime, Order::Desc)
            .one(db::pool())
            .await?
            .map(|news| format_datetime(news.publish_time));
        let games = load_source_games(&source.name, game).await?;

        Ok(Self {
            source_id: source.name,
            display_name: source.display_name,
            description: source.description,
            news_count,
            latest_published_at,
            games,
        })
    }
}

async fn load_source_games(
    source: &str,
    game: Option<&str>,
) -> Result<Vec<NewsSourceGameSummary>, sea_orm::DbErr> {
    let mut games_query = games::Entity::find();
    if let Some(game) = game {
        games_query = games_query.filter(games::Column::GameCode.eq(game));
    }
    let games = games_query
        .order_by(games::Column::Index, Order::Asc)
        .all(db::pool())
        .await?;
    let mut data = Vec::new();

    for game in games {
        let news_count = news_items::Entity::find()
            .filter(news_items::Column::Source.eq(source))
            .filter(news_items::Column::GameCode.eq(&game.game_code))
            .count(db::pool())
            .await?;

        if news_count == 0 {
            continue;
        }

        data.push(NewsSourceGameSummary {
            game_code: game.game_code,
            name_zh: game.name_zh,
            news_count,
        });
    }

    Ok(data)
}

fn format_datetime(value: DateTime<FixedOffset>) -> String {
    let offset = FixedOffset::east_opt(8 * 3600).expect("valid timezone offset");
    value.with_timezone(&offset).to_rfc3339()
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻来源在单个游戏下的数据覆盖情况。")]
pub struct NewsSourceGameSummary {
    /// 游戏代码。
    game_code: String,
    /// 游戏中文名称。
    name_zh: String,
    /// 当前来源在该游戏下的新闻数量。
    news_count: u64,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻来源列表响应。")]
pub struct NewsSourcesResponse {
    /// 新闻来源列表。
    sources: Vec<NewsSourceSummary>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻来源接口错误响应。")]
pub struct NewsSourceErrorResponse {
    /// 错误信息。
    message: String,
}
