use axum::{
    Router,
    http::{HeaderMap, HeaderValue, header},
    response::IntoResponse,
    routing::get,
};
use utoipa::openapi::OpenApi;
use utoipa_scalar::{Scalar, Servable};

pub fn router(api: OpenApi) -> Router {
    let scalar = Scalar::with_url("/scalar", api);
    let html = scalar.to_html();

    Router::new()
        .merge(scalar.custom_html(html.replace(
            "<title>Scalar</title>",
            format!("<title>{} - Scalar</title>\n<link rel=\"icon\" type=\"image/svg+xml\" href=\"/favicon.svg\" />", env!("CARGO_PKG_NAME")).as_str(),
        )))
        .merge(Router::new().route("/favicon.svg", get(favicon)))
}

pub async fn favicon() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("image/svg+xml"),
    );
    (headers, include_str!("../../assets/favicon.svg"))
}
