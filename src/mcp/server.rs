use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::protocol::{JsonRpcRequest, JsonRpcResponse};
use super::tools::ToolManager;
use crate::index::IndexEngine;

pub struct McpServer {
    tool_manager: ToolManager,
}

impl McpServer {
    pub fn new(engine: Arc<IndexEngine>) -> Self {
        Self {
            tool_manager: ToolManager::new(engine),
        }
    }

    pub async fn run_stdio(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin).lines();

        while let Some(line) = reader.next_line().await? {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }

            let req: JsonRpcRequest = match serde_json::from_str(line_trimmed) {
                Ok(r) => r,
                Err(e) => {
                    let err_resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
                    let resp_str = serde_json::to_string(&err_resp)?;
                    stdout.write_all(resp_str.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                    continue;
                }
            };

            let resp = self.handle_request(req).await;

            if let Some(r) = resp {
                let resp_str = serde_json::to_string(&r)?;
                stdout.write_all(resp_str.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        }

        Ok(())
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let method = req.method.as_str();
        let id = req.id.clone();

        match method {
            "initialize" => {
                let result = json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    },
                    "serverInfo": {
                        "name": "codebase-rag-2.0",
                        "version": "2.0.0"
                    }
                });
                Some(JsonRpcResponse::success(id, result))
            }
            "notifications/initialized" | "initialized" => {
                // Client initialized notification, no response required for JSON-RPC notifications
                None
            }
            "ping" => {
                Some(JsonRpcResponse::success(id, json!({})))
            }
            "tools/list" => {
                let tools = self.tool_manager.list_tools();
                let result = json!({
                    "tools": tools
                });
                Some(JsonRpcResponse::success(id, result))
            }
            "tools/call" => {
                let default_params = json!({});
                let default_args = json!({});
                let params = req.params.as_ref().unwrap_or(&default_params);
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let arguments = params.get("arguments").unwrap_or(&default_args);

                let call_result = self.tool_manager.call_tool(tool_name, arguments).await;
                let result_json = serde_json::to_value(call_result).unwrap_or(json!({}));

                Some(JsonRpcResponse::success(id, result_json))
            }
            _ => {
                if id.is_none() {
                    None
                } else {
                    Some(JsonRpcResponse::error(
                        id,
                        -32601,
                        format!("Method not found: {}", method),
                    ))
                }
            }
        }
    }
}
