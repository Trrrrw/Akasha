use std::time::Duration;

use axum::{
    Router,
    http::{
        HeaderValue, Method, Request,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    },
};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

pub fn apply(router: Router) -> Router {
    // init trace layer
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("-");

            tracing::info_span!(
                "request",
                request_id = %request_id,
                method = %request.method(),
                uri = %request.uri(),
                version = ?request.version(),
            )
        })
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(tower_http::LatencyUnit::Millis),
        );

    // cors
    let cors_layer = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://127.0.0.1:5173"),
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://100.78.33.96:5173"),
            HeaderValue::from_static("http://192.168.1.3:5173"),
            HeaderValue::from_static("https://varchive.trrw.cn"),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([ACCEPT, AUTHORIZATION, CONTENT_TYPE])
        .max_age(Duration::from_secs(3600));

    router.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .layer(trace_layer)
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(cors_layer),
    )
}
