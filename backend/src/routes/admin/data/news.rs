use axum::{Json, extract::Path, http::StatusCode};
use chrono::{DateTime, FixedOffset};
use db::entities::{
    games, news_categories, news_categories_link, news_items, news_sources, news_tags_link, tags,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

type AdminNewsResult<T> = Result<T, (StatusCode, Json<AdminNewsErrorResponse>)>;

pub async fn post(
    Json(req): Json<CreateNewsRequest>,
) -> AdminNewsResult<(StatusCode, Json<AdminNewsSuccessResponse>)> {
    let game_code = resolve_game_code(&req.game_code).await?;

    if news_items::Entity::find_by_id((
        req.remote_id.clone(),
        game_code.clone(),
        req.source.clone(),
    ))
    .one(db::pool())
    .await
    .map_err(internal_error)?
    .is_some()
    {
        return Err(error(StatusCode::CONFLICT, "news item already exists"));
    }

    news_sources::Entity::create_if_not_exists(news_sources::Model {
        name: req.source.clone(),
        display_name: req.source.clone(),
        description: String::new(),
    })
    .await
    .map_err(internal_error)?;

    let publish_time = parse_datetime(&req.publish_time)?;

    news_items::ActiveModel {
        remote_id: Set(req.remote_id.clone()),
        game_code: Set(game_code.clone()),
        source: Set(req.source.clone()),
        title: Set(req.title),
        intro: Set(req.intro),
        publish_time: Set(publish_time),
        source_url: Set(req.source_url),
        cover: Set(req.cover),
        is_video: Set(req.is_video),
        video_url: Set(req.video_url),
        raw_data: Set(req
            .raw_data
            .unwrap_or_else(|| Value::Object(Default::default()))
            .to_string()),
        ..Default::default()
    }
    .insert(db::pool())
    .await
    .map_err(internal_error)?;

    replace_categories(
        &req.remote_id,
        &game_code,
        &req.source,
        req.categories.unwrap_or_default(),
    )
    .await?;
    replace_tags(
        &req.remote_id,
        &game_code,
        &req.source,
        req.tags.unwrap_or_default(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(AdminNewsSuccessResponse { success: true }),
    ))
}

pub async fn put(
    Path(path): Path<NewsPath>,
    Json(req): Json<UpdateNewsRequest>,
) -> AdminNewsResult<Json<AdminNewsSuccessResponse>> {
    let Some(news) = news_items::Entity::find_by_id((
        path.remote_id.clone(),
        path.game_code.clone(),
        path.source.clone(),
    ))
    .one(db::pool())
    .await
    .map_err(internal_error)?
    else {
        return Err(not_found());
    };

    let mut active = news.into_active_model();

    if let Some(value) = req.title {
        active.title = Set(value);
    }
    if let Some(value) = req.intro {
        active.intro = Set(value);
    }
    if let Some(value) = req.publish_time {
        active.publish_time = Set(parse_datetime(&value)?);
    }
    if let Some(value) = req.source_url {
        active.source_url = Set(value);
    }
    if let Some(value) = req.cover {
        active.cover = Set(value);
    }
    if let Some(value) = req.is_video {
        active.is_video = Set(value);
    }
    if let Some(value) = req.video_url {
        active.video_url = Set(value);
    }
    if let Some(value) = req.raw_data {
        active.raw_data = Set(value.to_string());
    }

    active.update(db::pool()).await.map_err(internal_error)?;

    if let Some(categories) = req.categories {
        replace_categories(&path.remote_id, &path.game_code, &path.source, categories).await?;
    }
    if let Some(tags) = req.tags {
        replace_tags(&path.remote_id, &path.game_code, &path.source, tags).await?;
    }

    Ok(Json(AdminNewsSuccessResponse { success: true }))
}

pub async fn delete(Path(path): Path<NewsPath>) -> AdminNewsResult<Json<AdminNewsSuccessResponse>> {
    news_categories_link::Entity::delete_many()
        .filter(news_categories_link::Column::NewsRemoteId.eq(&path.remote_id))
        .filter(news_categories_link::Column::NewsGameBelong.eq(&path.game_code))
        .filter(news_categories_link::Column::NewsSourceBelong.eq(&path.source))
        .exec(db::pool())
        .await
        .map_err(internal_error)?;

    news_tags_link::Entity::delete_many()
        .filter(news_tags_link::Column::NewsRemoteId.eq(&path.remote_id))
        .filter(news_tags_link::Column::NewsGameBelong.eq(&path.game_code))
        .filter(news_tags_link::Column::NewsSourceBelong.eq(&path.source))
        .exec(db::pool())
        .await
        .map_err(internal_error)?;

    let result = news_items::Entity::delete_by_id((path.remote_id, path.game_code, path.source))
        .exec(db::pool())
        .await
        .map_err(internal_error)?;

    if result.rows_affected == 0 {
        return Err(not_found());
    }

    Ok(Json(AdminNewsSuccessResponse { success: true }))
}

async fn replace_categories(
    remote_id: &str,
    game_code: &str,
    source: &str,
    categories: Vec<String>,
) -> AdminNewsResult<()> {
    news_categories_link::Entity::delete_many()
        .filter(news_categories_link::Column::NewsRemoteId.eq(remote_id))
        .filter(news_categories_link::Column::NewsGameBelong.eq(game_code))
        .filter(news_categories_link::Column::NewsSourceBelong.eq(source))
        .exec(db::pool())
        .await
        .map_err(internal_error)?;

    for category in normalize_list(categories) {
        news_categories::Entity::create_if_not_exists(news_categories::Model {
            title: category.clone(),
            game_code: game_code.to_string(),
            index: 0,
        })
        .await
        .map_err(internal_error)?;

        news_categories_link::Entity::create_if_not_exists(news_categories_link::Model {
            news_remote_id: remote_id.to_string(),
            news_game_belong: game_code.to_string(),
            news_source_belong: source.to_string(),
            category_title: category,
            category_game_belong: game_code.to_string(),
        })
        .await
        .map_err(internal_error)?;
    }

    Ok(())
}

async fn replace_tags(
    remote_id: &str,
    game_code: &str,
    source: &str,
    tags: Vec<String>,
) -> AdminNewsResult<()> {
    news_tags_link::Entity::delete_many()
        .filter(news_tags_link::Column::NewsRemoteId.eq(remote_id))
        .filter(news_tags_link::Column::NewsGameBelong.eq(game_code))
        .filter(news_tags_link::Column::NewsSourceBelong.eq(source))
        .exec(db::pool())
        .await
        .map_err(internal_error)?;

    for tag in normalize_list(tags) {
        tags::Entity::create_if_not_exists(tags::Model {
            title: tag.clone(),
            game_code: game_code.to_string(),
        })
        .await
        .map_err(internal_error)?;

        news_tags_link::Entity::create_if_not_exists(news_tags_link::Model {
            news_remote_id: remote_id.to_string(),
            news_game_belong: game_code.to_string(),
            news_source_belong: source.to_string(),
            tag_title: tag,
            tag_game_belong: game_code.to_string(),
        })
        .await
        .map_err(internal_error)?;
    }

    Ok(())
}

async fn resolve_game_code(input: &str) -> AdminNewsResult<String> {
    games::Entity::resolve_game_code(input)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| error(StatusCode::BAD_REQUEST, "invalid game code"))
}

fn parse_datetime(value: &str) -> AdminNewsResult<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value)
        .map_err(|_| error(StatusCode::BAD_REQUEST, "invalid datetime"))
}

fn normalize_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .fold(Vec::new(), |mut values, value| {
            if !values.contains(&value) {
                values.push(value);
            }
            values
        })
}

fn not_found() -> (StatusCode, Json<AdminNewsErrorResponse>) {
    error(StatusCode::NOT_FOUND, "news item not found")
}

fn internal_error(err: sea_orm::DbErr) -> (StatusCode, Json<AdminNewsErrorResponse>) {
    error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn error(
    status: StatusCode,
    message: impl Into<String>,
) -> (StatusCode, Json<AdminNewsErrorResponse>) {
    (
        status,
        Json(AdminNewsErrorResponse {
            message: message.into(),
        }),
    )
}

#[derive(Deserialize)]
pub struct NewsPath {
    source: String,
    game_code: String,
    remote_id: String,
}

#[derive(Deserialize)]
pub struct CreateNewsRequest {
    remote_id: String,
    game_code: String,
    source: String,
    title: String,
    intro: Option<String>,
    publish_time: String,
    source_url: String,
    cover: String,
    is_video: bool,
    video_url: Option<String>,
    raw_data: Option<Value>,
    categories: Option<Vec<String>>,
    tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct UpdateNewsRequest {
    title: Option<String>,
    intro: Option<Option<String>>,
    publish_time: Option<String>,
    source_url: Option<String>,
    cover: Option<String>,
    is_video: Option<bool>,
    video_url: Option<Option<String>>,
    raw_data: Option<Value>,
    categories: Option<Vec<String>>,
    tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct AdminNewsSuccessResponse {
    success: bool,
}

#[derive(Serialize)]
pub struct AdminNewsErrorResponse {
    message: String,
}
