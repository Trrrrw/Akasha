use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

use crate::AxumMCP;

pub async fn post_handler(State(rust_mcp): State<AxumMCP>, Json(body): Json<Value>) -> Response {
    let id = body.get("id").cloned().unwrap_or(Value::Null);
    let method = body.get("method").and_then(|v| v.as_str()).unwrap_or("");

    if id.is_null() && method.starts_with("notifications/") {
        return StatusCode::ACCEPTED.into_response();
    }

    let response = match method {
        "initialize" => rust_mcp.initialize(id),
        "ping" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        }),
        "tools/list" => rust_mcp.tools_list(id),
        "tools/call" => rust_mcp.tools_call(id, &body).await,
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {method}")
            }
        }),
    };

    (StatusCode::OK, Json(response)).into_response()
}
