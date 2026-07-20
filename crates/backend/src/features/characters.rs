pub(crate) mod admin;
mod dto;
pub(crate) mod endpoints;
mod query;
pub(crate) mod use_cases;

use axum::{Router, routing::post};
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    use utoipa_axum::routes;

    OpenApiRouter::new().routes(routes!(endpoints::list))
}

pub(crate) fn admin_router() -> Router<AppState> {
    Router::new().route("/chars/sync", post(admin::sync))
}
