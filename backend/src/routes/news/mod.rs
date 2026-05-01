pub mod categories;
pub mod items;
pub mod meta;
pub mod rss;
pub mod search;
pub mod sources;

use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter {
    let sub_router = OpenApiRouter::new()
        .routes(routes!(meta::news_sync_status))
        .routes(routes!(items::news_items))
        .routes(routes!(search::news_search))
        .routes(routes!(rss::news_rss))
        .routes(routes!(items::news_item))
        .routes(routes!(items::news_item_related))
        .routes(routes!(categories::news_categories))
        .routes(routes!(sources::news_sources));
    OpenApiRouter::new().nest("/news", sub_router)
}
