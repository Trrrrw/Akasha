use axum::Router;
use axum_mcp::{AxumMCP, MCPServerInfo};

pub fn router() -> Router {
    let server_info = MCPServerInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        title: "".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: env!("CARGO_PKG_DESCRIPTION").to_string(),
        website_url: "".to_string(),
    };
    let instructions = "".to_string();
    let axum_mcp = AxumMCP {
        server_info,
        instructions,
    };

    axum_mcp.router()
}
