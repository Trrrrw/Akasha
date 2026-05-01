use serde::Serialize;
use serde_json::{Value, json};
use std::{future::Future, pin::Pin};

use crate::AxumMCP;

pub type MCPCallFuture = Pin<Box<dyn Future<Output = Value> + Send>>;

/// MCP 服务器基本信息
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MCPServerInfo {
    pub name: String,
    pub title: String,
    pub version: String,
    pub description: String,
    pub website_url: String,
}

impl Default for MCPServerInfo {
    fn default() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_string(),
            title: "".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            website_url: "".to_string(),
        }
    }
}

/// MCP 工具信息
pub struct MCPTool {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: fn() -> Value,
    pub call: fn(Value) -> MCPCallFuture,
}
crate::inventory::collect!(MCPTool);

impl AxumMCP {
    pub fn initialize(&self, id: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2025-11-25",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": &self.server_info,
                "instructions": &self.instructions
            }
        })
    }

    pub fn tools_list(&self, id: Value) -> Value {
        let tools: Vec<Value> = crate::inventory::iter::<MCPTool>
            .into_iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": (tool.input_schema)()
                })
            })
            .collect();

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": tools
            }
        })
    }

    pub async fn tools_call(&self, id: Value, body: &Value) -> Value {
        let tool_name = body
            .get("params")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let args = body
            .get("params")
            .and_then(|v| v.get("arguments"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        if let Some(tool) = crate::inventory::iter::<MCPTool>
            .into_iter()
            .find(|tool| tool.name == tool_name)
        {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": (tool.call)(args).await
            })
        } else {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32602,
                    "message": format!("Unknown tool: {tool_name}")
                }
            })
        }
    }
}
