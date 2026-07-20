pub(crate) mod admin;
mod dto;
pub(crate) mod endpoints;
mod query;
mod rss;
pub(crate) mod use_cases;

use axum::{Router, routing::post};
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    use utoipa_axum::routes;

    OpenApiRouter::new()
        .routes(routes!(endpoints::list_sources))
        .routes(routes!(endpoints::list_tags))
        .routes(routes!(endpoints::list))
        .routes(routes!(endpoints::detail))
        .routes(routes!(endpoints::rss))
}

pub(crate) fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/news/update", post(admin::update_news))
        .route("/news/tags/sync", post(admin::sync_tags))
        .route("/news/tags/update", post(admin::update_tags))
}
