use std::time::Duration;

use axum::{
    Router,
    http::{
        Method, Request,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    },
};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

/// 应用通用请求 ID、追踪和 CORS 中间件
pub fn apply(router: Router) -> Router {
    // 初始化请求追踪层
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
                // 查询参数可能含有 OAuth code 等敏感值，日志只记录路径
                path = %request.uri().path(),
                version = ?request.version(),
            )
        })
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(tower_http::LatencyUnit::Millis),
        );

    // 配置跨域访问规则
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
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
