mod status;
mod time;

use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter {
    let sub_router = OpenApiRouter::new()
        .routes(routes!(status::status))
        .routes(routes!(time::time));
    OpenApiRouter::new().nest("/system", sub_router)
}
