mod characters;
mod news;

use axum::{
    Router, middleware,
    routing::{delete, post, put},
};

use super::middleware::admin_auth;

pub fn router() -> Router {
    let data_router = Router::new()
        .route("/news", post(news::post))
        .route("/news/{source}/{game_code}/{remote_id}", put(news::put))
        .route(
            "/news/{source}/{game_code}/{remote_id}",
            delete(news::delete),
        )
        .route("/characters", post(characters::post))
        .route(
            "/characters/{game_code}/{character_code}",
            put(characters::put),
        )
        .route(
            "/characters/{game_code}/{character_code}",
            delete(characters::delete),
        )
        .route_layer(middleware::from_fn(admin_auth));

    Router::new().nest("/data", data_router)
}
