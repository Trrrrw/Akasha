mod login;
mod logout;
mod me;
mod refresh;
mod setup;
mod setup_status;

use axum::{
    Router, middleware,
    routing::{get, post},
};

use super::middleware::admin_auth;

pub fn router() -> Router {
    let protected = Router::new()
        .route("/logout", post(logout::post))
        .route("/me", get(me::get))
        .route_layer(middleware::from_fn(admin_auth));

    let auth_router = Router::new()
        .route("/setup", post(setup::post))
        .route("/setup-status", get(setup_status::get))
        .route("/login", post(login::post))
        .route("/refresh", post(refresh::post))
        .merge(protected);
    Router::new().nest("/auth", auth_router)
}
