use axum::Router;
use utoipa::openapi::{ContactBuilder, LicenseBuilder, OpenApi};
use utoipa_axum::router::OpenApiRouter;

use crate::{
    api::docs::OPENAPI_TITLE,
    features::{calendar, characters, events, games, news},
    state::AppState,
};

pub(crate) fn router() -> (Router<AppState>, OpenApi) {
    let (router, mut api) = OpenApiRouter::new()
        .nest(
            "/api/v1",
            OpenApiRouter::new()
                .merge(games::public_router())
                .merge(news::public_router())
                .merge(characters::public_router())
                .merge(events::public_router())
                .merge(calendar::public_router()),
        )
        .split_for_parts();

    setup_api_info(&mut api);

    (router, api)
}

fn setup_api_info(api: &mut OpenApi) {
    api.info.title = OPENAPI_TITLE.to_owned();
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
