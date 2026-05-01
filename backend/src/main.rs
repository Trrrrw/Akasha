mod routes;

use axum::Router;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt};
use utoipa::openapi::{ContactBuilder, LicenseBuilder, OpenApi};
use utoipa_axum::router::OpenApiRouter;

#[tokio::main]
async fn main() {
    init_tracing();
    db::init_and_sync().await;

    let (api_router, mut api) = OpenApiRouter::new()
        .merge(routes::root::router())
        .merge(routes::games::router())
        .merge(routes::news::router())
        .merge(routes::system::router())
        .split_for_parts();
    set_api_info(&mut api);

    let app = Router::new()
        .merge(api_router)
        .merge(routes::scalar::router(api))
        .merge(routes::mcp::router())
        .layer(
            TraceLayer::new_for_http()
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );
    let listener = TcpListener::bind("0.0.0.0:7040").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

fn set_api_info(api: &mut OpenApi) {
    api.info.title = env!("CARGO_PKG_NAME").to_string();
    api.info.version = env!("CARGO_PKG_VERSION").to_string();
    api.info.description = Some(env!("CARGO_PKG_DESCRIPTION").to_string());
    let author = env!("CARGO_PKG_AUTHORS");
    let (name, email) = match author.split_once('<') {
        Some((name, rest)) => {
            let email = rest.trim_end_matches('>').trim();
            (Some(name.trim()), Some(email))
        }
        None => (Some(author), None),
    };
    api.info.contact = Some(
        ContactBuilder::new()
            .name(name.filter(|value| !value.is_empty()))
            .email(email.filter(|value| !value.is_empty()))
            .build(),
    );
    api.info.license = Some(
        LicenseBuilder::new()
            .name(env!("CARGO_PKG_LICENSE"))
            .build(),
    );
}
