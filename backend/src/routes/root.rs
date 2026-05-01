use axum::{response::Redirect, routing::get};
use utoipa_axum::router::OpenApiRouter;

pub fn router() -> OpenApiRouter {
    OpenApiRouter::new().route("/", get(|| async { Redirect::permanent("/scalar") }))
}
