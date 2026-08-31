use std::path::PathBuf;

use axum::{
    Router,
    http::{HeaderValue, header::CACHE_CONTROL},
    response::{Redirect, Response},
    routing::get,
};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

const STATIC_CACHE_CONTROL: &str = "public, max-age=86400";

/// 构建公开静态资源路由
pub fn router<S>(game_data_asset_dir: PathBuf) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let static_assets = Router::new()
        // 显式提供站点图标，供浏览器和 RSS 阅读器读取
        .route_service("/favicon.ico", ServeFile::new("assets/favicon.ico"))
        .route_service("/robots.txt", ServeFile::new("assets/robots.txt"))
        .nest_service("/assets/game-data", ServeDir::new(game_data_asset_dir))
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            |response: &Response| {
                response
                    .status()
                    .is_success()
                    .then(|| HeaderValue::from_static(STATIC_CACHE_CONTROL))
            },
        ));

    Router::new().route("/", get(root)).merge(static_assets)
}

/// 在独立前端移除后将站点根路径引导至 API 文档
async fn root() -> Redirect {
    Redirect::permanent("/scalar")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode, header::CACHE_CONTROL},
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::{STATIC_CACHE_CONTROL, router};

    #[tokio::test]
    async fn adds_cache_control_to_static_assets() {
        let asset_dir = std::env::temp_dir().join(format!("akasha-assets-{}", Uuid::new_v4()));
        fs::create_dir_all(&asset_dir).expect("应创建临时静态资源目录");
        fs::write(asset_dir.join("sample.avif"), b"test").expect("应写入临时静态资源");

        let app: Router = router(asset_dir.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/game-data/sample.avif")
                    .body(Body::empty())
                    .expect("应构造请求"),
            )
            .await
            .expect("静态资源请求应成功");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CACHE_CONTROL], STATIC_CACHE_CONTROL);

        fs::remove_dir_all(asset_dir).expect("应清理临时静态资源目录");
    }

    #[tokio::test]
    async fn does_not_add_static_cache_control_to_root_redirect() {
        let app: Router = router(std::env::temp_dir());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("应构造请求"),
            )
            .await
            .expect("根路径请求应成功");

        assert!(response.headers().get(CACHE_CONTROL).is_none());
    }

    #[tokio::test]
    async fn does_not_cache_missing_static_assets() {
        let app: Router = router(std::env::temp_dir());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/game-data/missing.avif")
                    .body(Body::empty())
                    .expect("应构造请求"),
            )
            .await
            .expect("静态资源请求应返回响应");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.headers().get(CACHE_CONTROL).is_none());
    }
}
