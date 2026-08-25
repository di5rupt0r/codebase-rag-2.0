use codebase_rag::config::AppConfig;
use codebase_rag::index::IndexEngine;
use codebase_rag::mcp::protocol::JsonRpcRequest;
use codebase_rag::mcp::McpServer;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_mcp_initialize_and_tools_list() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = AppConfig::new(temp_dir.path());
    let engine = Arc::new(IndexEngine::new_in_ram(config).unwrap());
    let server = McpServer::new(engine);

    // 1. Initialize
    let init_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "1.0" }
        })),
    };

    let init_resp = server.handle_request(init_req).await.unwrap();
    assert_eq!(init_resp.id, Some(json!(1)));
    let result = init_resp.result.unwrap();
    assert_eq!(result["serverInfo"]["name"], "codebase-rag-2.0");
    assert_eq!(result["serverInfo"]["version"], "2.0.0");

    // 2. Tools List
    let list_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "tools/list".to_string(),
        params: None,
    };

    let list_resp = server.handle_request(list_req).await.unwrap();
    assert_eq!(list_resp.id, Some(json!(2)));
    let list_result = list_resp.result.unwrap();
    let tools = list_result["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 7);

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(tool_names.contains(&"search_codebase"));
    assert!(tool_names.contains(&"find_symbols"));
    assert!(tool_names.contains(&"get_repo_map"));
    assert!(tool_names.contains(&"search_files"));
    assert!(tool_names.contains(&"read_snippet"));
    assert!(tool_names.contains(&"index_codebase"));
    assert!(tool_names.contains(&"index_status"));
}

#[tokio::test]
async fn test_mcp_tool_call_index_and_search() {
    let temp_dir = tempfile::tempdir().unwrap();
    let sample_file = temp_dir.path().join("calculator.rs");
    std::fs::write(
        &sample_file,
        r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
    )
    .unwrap();

    let config = AppConfig::new(temp_dir.path());
    let engine = Arc::new(IndexEngine::new_in_ram(config).unwrap());
    let server = McpServer::new(engine);

    // Index
    let index_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(10)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "index_codebase",
            "arguments": {}
        })),
    };

    let index_resp = server.handle_request(index_req).await.unwrap();
    assert!(index_resp.error.is_none());

    // Search
    let search_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(11)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "search_codebase",
            "arguments": {
                "query": "multiply"
            }
        })),
    };

    let search_resp = server.handle_request(search_req).await.unwrap();
    assert!(search_resp.error.is_none());
    let search_result = search_resp.result.unwrap();
    let content_text = search_result["content"][0]["text"].as_str().unwrap();
    assert!(content_text.contains("multiply"));
}
