use axum::Router;
use utoipa::openapi::{ContactBuilder, LicenseBuilder, OpenApi};
use utoipa_axum::router::OpenApiRouter;

use crate::{
    api::docs::OPENAPI_TITLE,
    features::{calendar, game_data, games, news},
    state::AppState,
};

/// 构建公开的 v1 路由及其 OpenAPI 规范
pub(crate) fn router() -> (Router<AppState>, OpenApi) {
    let (router, mut api) = OpenApiRouter::new()
        .nest(
            "/api/v1",
            OpenApiRouter::new()
                .merge(games::public_router())
                .merge(game_data::public_router())
                .merge(news::public_router())
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

        assert!(paths.contains_key("/api/v1/games/{game_id}/news/series/{tag_name}/media/nfo"));
        assert!(paths.contains_key(
            "/api/v1/games/{game_id}/news/series/{tag_name}/episodes/{news_id}/media/nfo"
        ));
        assert!(paths.contains_key("/api/v1/games/{game_id}/data/{collection}"));
        assert!(!paths.keys().any(|path| path.ends_with("/data/character")));
        assert!(!paths.contains_key("/api/v1/games/ys/characters"));
        assert!(paths.contains_key("/api/v1/games/{game_id}/calendar/character-birthdays"));
        assert!(paths.contains_key("/api/v1/games/{game_id}/calendar/character-birthdays.ics"));
        assert!(paths.contains_key("/api/v1/games/{game_id}/calendar/events"));
        assert!(paths.contains_key("/api/v1/games/{game_id}/calendar/events.ics"));
        assert!(!paths.contains_key("/api/v1/games/{game_id}/events"));
        assert!(!paths.contains_key("/api/v1/games/{game_id}/calendar"));
        assert!(!paths.contains_key("/api/v1/games/{game_id}/calendar/ics"));
        assert!(!paths.contains_key("/api/v1/games/{game_id}/chars"));
        assert!(!paths.keys().any(|path| path.contains("/admin/")));
        assert!(!paths.keys().any(|path| path.contains("/auth/")));

        let event_ics_parameters =
            paths["/api/v1/games/{game_id}/calendar/events.ics"]["get"]["parameters"]
                .as_array()
                .expect("活动 ICS 应记录查询参数");
        let event_ics_parameter_names = event_ics_parameters
            .iter()
            .filter_map(|parameter| parameter["name"].as_str())
            .collect::<Vec<_>>();
        for name in [
            "from",
            "to",
            "kind",
            "event_mode",
            "start_reminder_minutes",
            "end_reminder_minutes",
        ] {
            assert!(
                event_ics_parameter_names.contains(&name),
                "活动 ICS 缺少 {name} 参数"
            );
        }

        let birthday_ics_parameters =
            paths["/api/v1/games/{game_id}/calendar/character-birthdays.ics"]["get"]["parameters"]
                .as_array()
                .expect("角色生日 ICS 应记录查询参数");
        let birthday_ics_parameter_names = birthday_ics_parameters
            .iter()
            .filter_map(|parameter| parameter["name"].as_str())
            .collect::<Vec<_>>();
        for name in [
            "q",
            "birthday_month",
            "gender",
            "reminder_time",
            "reminder_minutes_before",
        ] {
            assert!(
                birthday_ics_parameter_names.contains(&name),
                "角色生日 ICS 缺少 {name} 参数"
            );
        }

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
            "/api/v1/games/{game_id}/news/{news_id}/media/video",
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
