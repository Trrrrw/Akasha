use crate::{features::auth::endpoints, state::AppState};
use axum::{
    Router,
    routing::{get, post},
};

pub(crate) fn router() -> Router<AppState> {
    Router::new().nest(
        "/api/v1/auth",
        Router::new()
            .route("/github", get(endpoints::github_login))
            .route("/callback/github", get(endpoints::github_callback))
            .route("/refresh", post(endpoints::refresh))
            .route("/logout", post(endpoints::logout))
            .route("/me", get(endpoints::me)),
    )
}
