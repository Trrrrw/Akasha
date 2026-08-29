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
        .routes(routes!(endpoints::character_birthdays_json))
        .routes(routes!(endpoints::character_birthdays_ics))
        .routes(routes!(events::events_json))
        .routes(routes!(events::events_ics))
}

/// 构建活动投影的受保护管理路由
pub(crate) fn admin_router() -> Router<AppState> {
    Router::new().route("/games/{game_id}/calendar/events", put(admin::sync_events))
}

/// 返回日历接口统一使用的中国标准时区
fn china_timezone() -> FixedOffset {
    FixedOffset::east_opt(8 * 60 * 60).expect("UTC+8 应为有效时区偏移")
}
