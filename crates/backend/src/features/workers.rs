mod dto;
pub(crate) mod endpoints;
mod use_cases;

use crate::state::AppState;
use axum::{Router, routing::post};

pub(crate) fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/workers/acquire", post(endpoints::acquire))
        .route("/workers/heartbeat", post(endpoints::heartbeat))
        .route("/workers/checkpoint", post(endpoints::checkpoint))
        .route("/workers/complete", post(endpoints::complete))
        .route("/workers/fail", post(endpoints::fail))
}
