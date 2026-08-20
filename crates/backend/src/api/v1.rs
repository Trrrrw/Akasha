use axum::Router;
use utoipa::openapi::{
    ComponentsBuilder, ContactBuilder, LicenseBuilder, OpenApi,
    security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
};
use utoipa_axum::router::OpenApiRouter;

use crate::{
    api::{auth, docs::OPENAPI_TITLE},
    features::{calendar, events, game_data, games, news},
    state::AppState,
};

/// 构建公开的 v1 路由及其 OpenAPI 规范
pub(crate) fn router() -> (Router<AppState>, OpenApi) {
    let (router, mut api) = OpenApiRouter::new()
        .nest(
            "/api/v1",
            OpenApiRouter::new()
                .merge(auth::router())
                .merge(games::public_router())
                .merge(game_data::public_router())
                .merge(news::public_router())
                .merge(events::public_router())
                .merge(calendar::public_router()),
        )
        .split_for_parts();

    setup_api_info(&mut api);

    (router, api)
}

/// 添加生成 OpenAPI 文档共用的包元数据
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

    // 集中声明认证接口使用的 Bearer 与 HttpOnly Cookie 凭据
    let components = api
        .components
        .get_or_insert_with(|| ComponentsBuilder::new().build());
    components.add_security_scheme(
        "access_token",
        SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Bearer)
                .bearer_format("JWT")
                .description(Some("通过刷新会话接口获取的短期访问令牌"))
                .build(),
        ),
    );
    components.add_security_scheme(
        "refresh_token",
        SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
            "akasha_refresh_token",
            "GitHub OAuth 登录成功后由后端写入的 HttpOnly 刷新令牌 Cookie",
        ))),
    );
    components.add_security_scheme(
        "oauth_state",
        SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
            "akasha_oauth_state",
            "开始 GitHub OAuth 登录时由后端写入的 HttpOnly 防伪 Cookie",
        ))),
    );
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::router;

    /// 公开 OpenAPI 只收录公开路由且每个操作都提供简洁说明
    #[test]
    fn documents_every_public_operation() {
        let (_, api) = router();
        let document = serde_json::to_value(api).expect("应序列化 OpenAPI 文档");
        let paths = document["paths"].as_object().expect("应包含 paths");

        assert!(paths.contains_key("/api/v1/games/{game_id}/news/series/{tag_name}/nfo"));
        assert!(
            paths.contains_key(
                "/api/v1/games/{game_id}/news/series/{tag_name}/episodes/{news_id}/nfo"
            )
        );
        assert!(paths.contains_key("/api/v1/auth/github"));
        assert!(paths.contains_key("/api/v1/auth/callback/github"));
        assert!(paths.contains_key("/api/v1/auth/refresh"));
        assert!(paths.contains_key("/api/v1/auth/logout"));
        assert!(paths.contains_key("/api/v1/auth/me"));
        assert!(paths.contains_key("/api/v1/games/{game_id}/data/{collection}"));
        assert!(!paths.keys().any(|path| path.ends_with("/data/character")));
        assert!(!paths.contains_key("/api/v1/games/ys/characters"));
        assert!(paths.contains_key("/api/v1/games/{game_id}/calendar/character-birthdays"));
        assert!(paths.contains_key("/api/v1/games/{game_id}/calendar/character-birthdays.ics"));
        assert!(!paths.contains_key("/api/v1/games/{game_id}/calendar"));
        assert!(!paths.contains_key("/api/v1/games/{game_id}/calendar/ics"));
        assert!(!paths.contains_key("/api/v1/games/{game_id}/chars"));
        assert!(!paths.keys().any(|path| path.contains("/admin/")));

        let security_schemes = document["components"]["securitySchemes"]
            .as_object()
            .expect("应包含认证安全方案");
        assert!(security_schemes.contains_key("access_token"));
        assert!(security_schemes.contains_key("refresh_token"));
        assert!(security_schemes.contains_key("oauth_state"));

        let me_security = document["paths"]["/api/v1/auth/me"]["get"]["security"]
            .as_array()
            .expect("当前用户接口应声明安全要求");
        assert!(
            me_security
                .iter()
                .any(|requirement| requirement.get("access_token").is_some())
        );

        let schemas = document["components"]["schemas"]
            .as_object()
            .expect("应包含响应模型");
        assert!(schemas.contains_key("GameResponse"));
        assert!(!schemas.contains_key("GameDetailResponse"));
        assert!(
            schemas["NewsItemResponse"]["properties"]
                .get("source")
                .is_some()
        );
        assert!(
            schemas["NewsItemResponse"]["properties"]
                .get("source_id")
                .is_none()
        );

        for path in [
            "/api/v1/games/{game_id}/news",
            "/api/v1/games/{game_id}/news/rss",
        ] {
            let parameters = document["paths"][path]["get"]["parameters"]
                .as_array()
                .expect("新闻查询应记录公共筛选参数");
            let names = parameters
                .iter()
                .filter_map(|parameter| parameter["name"].as_str())
                .collect::<Vec<_>>();
            for name in ["source", "q", "tag", "character"] {
                assert!(names.contains(&name), "GET {path} 缺少 {name} 参数");
            }
            for legacy_name in ["source_id", "tags", "during", "reverse"] {
                assert!(
                    !names.contains(&legacy_name),
                    "GET {path} 不应继续公开 {legacy_name} 参数"
                );
            }
        }

        for path in [
            "/api/v1/games/{game_id}/news/{news_id}/video",
            "/api/v1/games/{game_id}/news/rss",
        ] {
            let rate_limit_response = &document["paths"][path]["get"]["responses"]["429"];
            assert!(
                rate_limit_response["headers"].get("Retry-After").is_some(),
                "GET {path} 应记录限流重试请求头"
            );
        }

        for (path, path_item) in paths {
            for method in ["get", "post", "put", "patch", "delete"] {
                let Some(operation) = path_item.get(method) else {
                    continue;
                };
                assert_non_empty_text(operation, "summary", path, method);
                assert_non_empty_text(operation, "description", path, method);
            }
        }
    }

    /// 断言一个 OpenAPI 操作包含非空文本字段
    fn assert_non_empty_text(operation: &Value, field: &str, path: &str, method: &str) {
        let value = operation[field].as_str().unwrap_or_default().trim();
        assert!(!value.is_empty(), "{method} {path} 缺少 OpenAPI {field}");
    }
}
