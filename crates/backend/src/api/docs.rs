use axum::{Json, Router, routing::get};
use utoipa::openapi::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::state::AppState;

pub(super) const OPENAPI_TITLE: &str = "Akasha";

/// 返回当前公开接口的 OpenAPI JSON
async fn openapi_json(api: OpenApi) -> Json<OpenApi> {
    Json(api)
}

/// 构建交互式 OpenAPI 文档路由
pub fn router(api: OpenApi) -> Router<AppState> {
    let scalar = Scalar::with_url("/scalar", api.clone());
    let html = scalar.to_html();

    Router::new()
        .route(
            "/openapi.json",
            get(move || {
                let api = api.clone();
                openapi_json(api)
            }),
        )
        .merge(scalar.custom_html(html.replace(
            "<title>Scalar</title>",
            format!("<title>{} - Scalar</title>\n<link rel=\"icon\" type=\"image/svg+xml\" href=\"/assets/logo.svg\" />", OPENAPI_TITLE).as_str(),
        )))
}

#[cfg(test)]
mod tests {
    use axum::Json;

    use super::openapi_json;

    /// 公开 OpenAPI 规范应能直接序列化为 JSON
    #[tokio::test]
    async fn serializes_public_openapi_document() {
        let (_, api) = crate::api::v1::router();
        let Json(document) = openapi_json(api).await;
        let document = serde_json::to_value(document).expect("应序列化 OpenAPI JSON");

        assert!(document["paths"].get("/api/v1/games").is_some());
        assert!(
            document["paths"]
                .get("/api/v1/games/{game_id}/news/{news_id}/media/video")
                .is_some()
        );
        let paths = document["paths"].as_object().expect("应包含 paths");
        assert!(!paths.keys().any(|path| path.contains("/auth/")));
    }
}
