pub(crate) mod endpoints;

use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

/// 构建公开游戏活动路由
pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    use utoipa_axum::routes;

    OpenApiRouter::new().routes(routes!(endpoints::list))
}
