use axum::Router;
use tower_http::services::ServeDir;

pub fn router() -> Router {
    let sub_router = Router::new();
    Router::new()
        .nest_service("/admin", ServeDir::new("admin/dist"))
        .nest("/admin", sub_router)
}
