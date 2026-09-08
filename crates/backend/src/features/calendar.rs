pub(crate) mod admin;
pub(crate) mod endpoints;
mod events;
mod ics;

use axum::{Router, routing::put};
use chrono::FixedOffset;
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

/// 构建公开日历路由
pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    use utoipa_axum::routes;

    OpenApiRouter::new()
        .routes(routes!(events::calendar_capabilities))
        .routes(routes!(events::calendar_json))
        .routes(routes!(events::calendar_ics))
}

/// 构建日程投影的受保护管理路由
pub(crate) fn admin_router() -> Router<AppState> {
    Router::new().route(
        "/games/{game_id}/calendar/entries",
        put(admin::sync_entries),
    )
}

/// 返回日历接口统一使用的中国标准时区
fn china_timezone() -> FixedOffset {
    FixedOffset::east_opt(8 * 60 * 60).expect("UTC+8 应为有效时区偏移")
}
