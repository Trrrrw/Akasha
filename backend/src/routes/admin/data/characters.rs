use axum::{Json, extract::Path, http::StatusCode};
use chrono::{DateTime, FixedOffset};
use db::{
    entities::{game_characters, games},
    enums::gender::Gender,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, IntoActiveModel};
use serde::{Deserialize, Serialize};

type AdminCharacterResult<T> = Result<T, (StatusCode, Json<AdminCharacterErrorResponse>)>;

pub async fn post(
    Json(req): Json<CreateCharacterRequest>,
) -> AdminCharacterResult<(StatusCode, Json<AdminCharacterSuccessResponse>)> {
    let game_code = resolve_game_code(&req.game_code).await?;

    if game_characters::Entity::find_by_id((req.character_code.clone(), game_code.clone()))
        .one(db::pool())
        .await
        .map_err(internal_error)?
        .is_some()
    {
        return Err(error(StatusCode::CONFLICT, "character already exists"));
    }

    game_characters::ActiveModel {
        character_code: Set(req.character_code),
        game_code: Set(game_code),
        name: Set(req.name),
        birthday_month: Set(req.birthday_month),
        birthday_day: Set(req.birthday_day),
        release_time: Set(req
            .release_time
            .as_deref()
            .map(parse_datetime)
            .transpose()?),
        gender: Set(req.gender.as_deref().map(parse_gender).transpose()?),
        extra: Set(req.extra),
        ..Default::default()
    }
    .insert(db::pool())
    .await
    .map_err(internal_error)?;

    Ok((
        StatusCode::CREATED,
        Json(AdminCharacterSuccessResponse { success: true }),
    ))
}

pub async fn put(
    Path(path): Path<CharacterPath>,
    Json(req): Json<UpdateCharacterRequest>,
) -> AdminCharacterResult<Json<AdminCharacterSuccessResponse>> {
    let Some(character) =
        game_characters::Entity::find_by_id((path.character_code.clone(), path.game_code.clone()))
            .one(db::pool())
            .await
            .map_err(internal_error)?
    else {
        return Err(not_found());
    };

    let mut active = character.into_active_model();

    if let Some(value) = req.name {
        active.name = Set(value);
    }
    if let Some(value) = req.birthday_month {
        active.birthday_month = Set(value);
    }
    if let Some(value) = req.birthday_day {
        active.birthday_day = Set(value);
    }
    if let Some(value) = req.release_time {
        active.release_time = Set(value.as_deref().map(parse_datetime).transpose()?);
    }
    if let Some(value) = req.gender {
        active.gender = Set(value.as_deref().map(parse_gender).transpose()?);
    }
    if let Some(value) = req.extra {
        active.extra = Set(value);
    }

    active.update(db::pool()).await.map_err(internal_error)?;

    Ok(Json(AdminCharacterSuccessResponse { success: true }))
}

pub async fn delete(
    Path(path): Path<CharacterPath>,
) -> AdminCharacterResult<Json<AdminCharacterSuccessResponse>> {
    let result = game_characters::Entity::delete_by_id((path.character_code, path.game_code))
        .exec(db::pool())
        .await
        .map_err(internal_error)?;

    if result.rows_affected == 0 {
        return Err(not_found());
    }

    Ok(Json(AdminCharacterSuccessResponse { success: true }))
}

async fn resolve_game_code(input: &str) -> AdminCharacterResult<String> {
    games::Entity::resolve_game_code(input)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| error(StatusCode::BAD_REQUEST, "invalid game code"))
}

fn parse_datetime(value: &str) -> AdminCharacterResult<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value)
        .map_err(|_| error(StatusCode::BAD_REQUEST, "invalid datetime"))
}

fn parse_gender(value: &str) -> AdminCharacterResult<Gender> {
    match value.trim().to_lowercase().as_str() {
        "male" | "m" | "男" => Ok(Gender::Male),
        "female" | "f" | "女" => Ok(Gender::Female),
        "unknown" | "未知" => Ok(Gender::Unknown),
        _ => Err(error(StatusCode::BAD_REQUEST, "invalid gender")),
    }
}

fn not_found() -> (StatusCode, Json<AdminCharacterErrorResponse>) {
    error(StatusCode::NOT_FOUND, "character not found")
}

fn internal_error(err: sea_orm::DbErr) -> (StatusCode, Json<AdminCharacterErrorResponse>) {
    error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn error(
    status: StatusCode,
    message: impl Into<String>,
) -> (StatusCode, Json<AdminCharacterErrorResponse>) {
    (
        status,
        Json(AdminCharacterErrorResponse {
            message: message.into(),
        }),
    )
}

#[derive(Deserialize)]
pub struct CharacterPath {
    game_code: String,
    character_code: String,
}

#[derive(Deserialize)]
pub struct CreateCharacterRequest {
    character_code: String,
    game_code: String,
    name: String,
    birthday_month: Option<i16>,
    birthday_day: Option<i16>,
    release_time: Option<String>,
    gender: Option<String>,
    extra: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateCharacterRequest {
    name: Option<String>,
    birthday_month: Option<Option<i16>>,
    birthday_day: Option<Option<i16>>,
    release_time: Option<Option<String>>,
    gender: Option<Option<String>>,
    extra: Option<Option<String>>,
}

#[derive(Serialize)]
pub struct AdminCharacterSuccessResponse {
    success: bool,
}

#[derive(Serialize)]
pub struct AdminCharacterErrorResponse {
    message: String,
}
