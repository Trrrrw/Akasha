mod dto;
pub(crate) mod endpoints;
pub(crate) mod use_cases;

use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    use utoipa_axum::routes;

    OpenApiRouter::new()
        .routes(routes!(endpoints::list))
        .routes(routes!(endpoints::detail))
}
