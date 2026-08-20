pub(crate) mod endpoints;

use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

/// 构建公开日历路由
pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    use utoipa_axum::routes;

    OpenApiRouter::new()
        .routes(routes!(endpoints::character_birthdays_json))
        .routes(routes!(endpoints::character_birthdays_ics))
}
