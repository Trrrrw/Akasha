use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        .fallback_service(
            ServeDir::new("frontend/wiki/dist").fallback(ServeFile::new("wiki/dist/index.html")),
        )
}
