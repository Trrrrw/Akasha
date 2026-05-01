mod response;
mod sse;
mod streamable_http;

pub use axum;
use axum::{Router, routing::post};
pub use inventory;
pub use serde_json;

pub use axum_mcp_macros::{MCPInputSchema, mcp};
pub use response::{MCPCallFuture, MCPServerInfo, MCPTool};

pub trait MCPInputSchema {
    fn schema() -> serde_json::Value;
}

#[derive(Clone)]
pub struct AxumMCP {
    pub server_info: MCPServerInfo,
    pub instructions: String,
}

impl AxumMCP {
    pub fn router(self) -> Router {
        Router::new()
            .route(
                "/mcp",
                post(streamable_http::post_handler).get(sse::handler),
            )
            .with_state(self)
    }
}
