use axum::Router;
use utoipa::openapi::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::state::AppState;

pub(super) const OPENAPI_TITLE: &str = "Akasha";

pub fn router(api: OpenApi) -> Router<AppState> {
    let scalar = Scalar::with_url("/scalar", api);
    let html = scalar.to_html();

    Router::new()
        .merge(scalar.custom_html(html.replace(
            "<title>Scalar</title>",
            format!("<title>{} - Scalar</title>\n<link rel=\"icon\" type=\"image/svg+xml\" href=\"/assets/logo.svg\" />", OPENAPI_TITLE).as_str(),
        )))
}
