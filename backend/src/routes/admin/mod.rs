pub mod admin_token;
mod auth;
mod data;
mod middleware;
pub mod setup_token;

use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

pub fn router() -> Router {
    let sub_router = Router::new().merge(auth::router()).merge(data::router());
    Router::new().nest("/admin", sub_router).nest_service(
        "/admin",
        ServeDir::new("admin/dist").fallback(ServeFile::new("admin/dist/index.html")),
    )
}
