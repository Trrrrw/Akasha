use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn app() -> Router {
    app_with_search_burst(30).await
}

async fn app_with_search_burst(news_search_burst: u32) -> Router {
    crate::build_app(crate::Config {
        log_level: "error".into(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        asset_base_url: "https://api.example".into(),
        game_data_asset_dir: std::env::temp_dir().join("akasha-api-tests"),
        mys_cookie: None,
        database: akasha_db::DbOptions {
            sqlite_path: ":memory:".into(),
        },
        data_write_token: "test-only-write-token".into(),
        audit_log_retention_days: 30,
        public_rate_limits: crate::config::PublicRateLimitConfig {
            trusted_proxy_ips: vec![],
            video_requests_per_minute: 30,
            video_burst: 30,
            news_search_requests_per_minute: 30,
            news_search_burst,
            rss_requests_per_minute: 30,
            rss_burst: 30,
            rss_mys_refresh_limit: 1,
        },
    })
    .await
    .unwrap()
    .layer(axum::Extension(axum::extract::ConnectInfo(
        "127.0.0.1:12345".parse::<std::net::SocketAddr>().unwrap(),
    )))
}

#[tokio::test]
async fn character_resources_read_each_games_minimal_directory() {
    let app = app().await;
    for game in ["ys", "sr", "zzz"] {
        let path = format!("/api/v1/games/{game}/characters");
        let (status, page) = json_request(&app, "GET", &path, Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page["items"], json!([]));
        assert_eq!(page["total"], 0);
        let (status, _) = json_request(&app, "PUT", &format!("/api/v1/admin/games/{game}/data/character"), json!({
            "items":[{"id":"1","name":"测试角色","summary":{"id":"1","name":"测试角色","birthday_month":11,"birthday_day":13},"assets":{}}]
        })).await;
        assert_eq!(status, StatusCode::OK);
        let (_, page) = json_request(&app, "GET", &path, Value::Null).await;
        assert_eq!(
            page["items"][0],
            json!({"id":"1","name":"测试角色","icon":null,"birthday_month":11,"birthday_day":13})
        );
    }
}

#[tokio::test]
async fn news_collection_supports_an_unscoped_query() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/news")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(body["items"], json!([]));
}

async fn json_request(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("Authorization", "Bearer test-only-write-token")
                .header("Content-Type", "application/json")
                .body(if method == "GET" {
                    Body::empty()
                } else {
                    Body::from(body.to_string())
                })
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 262144).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn news_body_search_is_opt_in_and_returns_plain_highlighted_excerpts() {
    let app = app().await;
    let mut article = json!({
        "game_id":"ys","source_id":"web_cn","id":"body-search","title":"活动公告",
        "intro": format!("<p>{}</p><p>领取<span>原石</span>奖励 &amp; 道具</p>", "前文。".repeat(100)),
        "publish_time":"2026-01-01T00:00:00Z","source_url":"https://example.com/news",
        "news_type":"article","tags":[],"raw_data":{}
    });
    let (status, _) = json_request(&app, "POST", "/api/v1/admin/news", article.clone()).await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, title_only) = json_request(&app, "GET", "/api/v1/news?q=原石", Value::Null).await;
    assert_eq!(title_only["total"], 0);

    let (status, compact) = json_request(
        &app,
        "GET",
        "/api/v1/news/search?q=原石&search_body=true&game=ys&limit=1",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(compact["total"], 1);
    let hit = &compact["items"][0];
    assert_eq!(hit["id"], "body-search");
    assert!(hit["search_match"]["body"].is_array());
    assert!(hit.get("intro").is_none());
    assert!(hit.get("video_url").is_none());

    let (status, _) = json_request(&app, "GET", "/api/v1/news/search?q=%20", Value::Null).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (_, compact_title) =
        json_request(&app, "GET", "/api/v1/news/search?q=公告", Value::Null).await;
    assert_eq!(compact_title["total"], 1);
    assert!(compact_title["items"][0].get("search_match").is_none());

    let (_, detail) = json_request(
        &app,
        "GET",
        "/api/v1/news/ys/web_cn/body-search",
        Value::Null,
    )
    .await;
    assert_eq!(detail["intro"], article["intro"]);
    for (query, matched) in [
        ("原石", "原石"),
        ("原石奖励", "原石奖励"),
        ("公告%20原石", "原石"),
    ] {
        let path = format!("/api/v1/news?q={query}&search_body=true&game=ys");
        let (status, page) = json_request(&app, "GET", &path, Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page["total"], 1);
        let body = page["items"][0]["search_match"]["body"].as_array().unwrap();
        assert!(
            body.iter()
                .any(|part| part == &json!({"text":matched,"highlight":true}))
        );
        let text = body
            .iter()
            .map(|part| part["text"].as_str().unwrap())
            .collect::<String>();
        assert!(text.starts_with('…') && text.contains("领取原石奖励 & 道具"));
        assert!(!text.contains("<span>"));
        assert!(text.chars().count() < 180);
    }
    let (_, excluded) = json_request(
        &app,
        "GET",
        "/api/v1/news?q=公告%20-原石&search_body=true",
        Value::Null,
    )
    .await;
    assert_eq!(excluded["total"], 0);
    let (_, unfiltered) =
        json_request(&app, "GET", "/api/v1/news?search_body=true", Value::Null).await;
    assert!(unfiltered["items"][0].get("search_match").is_none());

    article["intro"] = json!("<p>获得限定道具</p>");
    let (status, _) = json_request(&app, "POST", "/api/v1/admin/news", article).await;
    assert_eq!(status, StatusCode::OK);
    let (_, stale) = json_request(
        &app,
        "GET",
        "/api/v1/news?q=原石奖励&search_body=true",
        Value::Null,
    )
    .await;
    assert_eq!(stale["total"], 0);
    let (_, updated) = json_request(
        &app,
        "GET",
        "/api/v1/news?q=限定道具&search_body=true",
        Value::Null,
    )
    .await;
    assert_eq!(updated["total"], 1);
}

#[tokio::test]
async fn news_search_shares_its_limit_with_legacy_search_only() {
    let app = app_with_search_burst(1).await;
    let (status, _) = json_request(&app, "GET", "/api/v1/news/search?q=原石", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let limited = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/news?q=原石")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(limited.headers().contains_key("retry-after"));

    let (status, _) = json_request(&app, "GET", "/api/v1/news", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let rss = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/news/rss?q=原石")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rss.status(), StatusCode::OK);
}

#[tokio::test]
async fn news_identity_filters_and_pagination_are_scoped_to_each_record() {
    let app = app().await;
    let (status, _) = json_request(
        &app,
        "POST",
        "/api/v1/admin/news/tags",
        json!({
            "game_id": "ys",
            "source_id": "web_cn",
            "tags": [{"name":"活动","index":1,"group":null,"group_index":null}]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = json_request(&app, "PUT", "/api/v1/admin/games/ys/data/character", json!({
        "items":[{"id":"1","name":"测试角色","summary":{"id":"1","name":"测试角色"},"assets":{}}]
    })).await;
    assert_eq!(status, StatusCode::OK);
    for (game, source, title, tags) in [
        ("ys", "web_cn", "原神", vec!["活动"]),
        ("sr", "web_cn", "星铁", vec![]),
        ("sr", "mys", "星铁米游社", vec![]),
    ] {
        let (status, _) = json_request(
            &app,
            "POST",
            "/api/v1/admin/news",
            json!({
                "game_id":game,"source_id":source,"id":"42","title":title,"intro":"正文",
                "publish_time":"2026-01-01T00:00:00Z","source_url":"https://example.com/news",
                "cover":null,"news_type":"article","video_url":null,"video_playback":null,
                "video_duration_ms":null,"tags":tags,"characters": if game == "ys" { json!([{"id":"1","name":"测试角色"}]) } else { json!([]) },"raw_data":{}
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }
    for (query, total) in [
        ("", 3),
        ("?game=sr", 2),
        ("?game=ys&game=sr&source=web_cn", 2),
        ("?tag=活动", 1),
        ("?untagged=true", 2),
        ("?character=1", 1),
    ] {
        let (status, page) =
            json_request(&app, "GET", &format!("/api/v1/news{query}"), Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page["total"], total);
        assert_eq!(page["items"].as_array().unwrap().len(), total as usize);
    }
    let (_, page) = json_request(&app, "GET", "/api/v1/news?limit=1&offset=1", Value::Null).await;
    assert_eq!(page["total"], 3);
    assert_eq!(page["items"][0]["game_id"], "sr");
    assert_eq!(page["items"][0]["source"], "web_cn");
    let (_, detail) = json_request(&app, "GET", "/api/v1/news/ys/web_cn/42", Value::Null).await;
    assert_eq!(detail["title"], "原神");
    assert_eq!(detail["game_id"], "ys");
    assert!(
        detail["cover"]
            .as_str()
            .unwrap()
            .contains("/assets/games/ys/")
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/news/rss")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let xml = String::from_utf8(
        to_bytes(response.into_body(), 262144)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    for guid in ["ys:web_cn:42", "sr:web_cn:42", "sr:mys:42"] {
        assert!(xml.contains(guid));
    }
}
