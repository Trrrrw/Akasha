use axum::{Json, http::StatusCode};
use chrono::{FixedOffset, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use axum_mcp::mcp;
use db::entities::{meta, news_items};

#[mcp]
#[utoipa::path(
    get,
    path = "/meta",
    tag = "News",
    summary = "获取新闻同步元信息",
    description = "返回爬虫最近一次成功完成时间，以及当前数据库中最新新闻的发布时间。",
    params(),
    responses(
        (status = 200, body = NewsMetaResponse),
        (status = 404, body = NewsMetaErrorResponse)
    )
)]
pub async fn news_sync_status()
-> Result<Json<NewsMetaResponse>, (StatusCode, Json<NewsMetaErrorResponse>)> {
    let offset = FixedOffset::east_opt(8 * 3600).unwrap();

    let last_crawler_at = match meta::Entity::get_value_by_key("last_crawler_at").await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(NewsMetaErrorResponse {
                    message: "last_crawler_at not found".to_string(),
                }),
            ));
        }
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(NewsMetaErrorResponse {
                    message: err.to_string(),
                }),
            ));
        }
    };

    let local_latest_news_publish_time =
        match news_items::Entity::get_local_latest_news(None, None).await {
            Ok(Some(n)) => n.publish_time,
            _ => Utc::now().into(),
        };

    Ok(Json(NewsMetaResponse {
        last_crawler_at,
        last_published_at: local_latest_news_publish_time
            .with_timezone(&offset)
            .to_rfc3339(),
    }))
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻同步元信息响应。")]
pub struct NewsMetaResponse {
    /// 爬虫最近一次成功完成时间，RFC3339 格式。
    last_crawler_at: String,
    /// 当前数据库中最新新闻的发布时间，RFC3339 格式。
    last_published_at: String,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "新闻同步元信息错误响应。")]
pub struct NewsMetaErrorResponse {
    /// 错误信息。
    message: String,
}
