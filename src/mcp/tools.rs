use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use strsim::jaro_winkler;

use super::protocol::{CallToolResult, ToolDefinition};
use crate::index::IndexEngine;
use crate::retrieval::{ContextPacker, HybridRetriever, RepoMapGenerator};

pub struct ToolManager {
    engine: Arc<IndexEngine>,
    retriever: HybridRetriever,
    repo_map_gen: RepoMapGenerator,
    context_packer: ContextPacker,
}

impl ToolManager {
    pub fn new(engine: Arc<IndexEngine>) -> Self {
        let tantivy_store = engine.get_tantivy_store();
        let vector_store = engine.get_vector_store();
        let symbol_store = engine.get_symbol_store();
        let embedding_engine = engine.get_embedding_engine();
        let config = engine.get_config().retrieval.clone();

        let retriever = HybridRetriever::new(
            tantivy_store.clone(),
            vector_store,
            symbol_store.clone(),
            embedding_engine,
            config.clone(),
        );

        let repo_map_gen = RepoMapGenerator::new(symbol_store);
        let context_packer = ContextPacker::new(config.max_context_tokens);

        Self {
            engine,
            retriever,
            repo_map_gen,
            context_packer,
        }
    }

    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "search_codebase".to_string(),
                description: "Perform modern hybrid retrieval (BM25 lexical + semantic vector + AST symbols fused via RRF) over the codebase to find relevant code snippets, definitions, and implementations.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Natural language or keyword query describing what you are looking for (e.g. 'authentication middleware', 'handle_user_login', 'database pool connection')"
                        },
                        "top_k": {
                            "type": "integer",
                            "description": "Maximum number of code snippets to return (default: 10)"
                        }
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "find_symbols".to_string(),
                description: "Search for exact or fuzzy AST symbol definitions (functions, structs, classes, traits, interfaces, methods) across the entire codebase.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "The symbol identifier or prefix to search for (e.g. 'IndexEngine', 'parse_symbols', 'UserProfile')"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of symbol matches to return (default: 15)"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
            ToolDefinition {
                name: "get_repo_map".to_string(),
                description: "Generate a compressed structural skeleton outline of the repository containing files and key symbol signatures to understand high-level codebase architecture.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "max_tokens": {
                            "type": "integer",
                            "description": "Maximum token budget for the skeleton map (default: 4000)"
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "search_files".to_string(),
                description: "Fuzzy search for file paths in the workspace ignoring gitignored files.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "File path pattern or query to match (e.g. 'config', 'tests/parser', 'main.rs')"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of matching file paths to return (default: 20)"
                        }
                    },
                    "required": ["pattern"]
                }),
            },
            ToolDefinition {
                name: "read_snippet".to_string(),
                description: "Read an exact line slice from a file in the workspace with line numbers.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative or absolute path to the file"
                        },
                        "start_line": {
                            "type": "integer",
                            "description": "Starting line number (1-indexed)"
                        },
                        "end_line": {
                            "type": "integer",
                            "description": "Ending line number (1-indexed, inclusive)"
                        }
                    },
                    "required": ["path", "start_line", "end_line"]
                }),
            },
            ToolDefinition {
                name: "index_codebase".to_string(),
                description: "Trigger full or incremental indexing of the codebase files, AST symbols, and embeddings.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolDefinition {
                name: "index_status".to_string(),
                description: "Get statistics on current indexed files, AST chunks, symbols, and storage location.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    pub async fn call_tool(&self, name: &str, arguments: &Value) -> CallToolResult {
        match name {
            "search_codebase" => self.handle_search_codebase(arguments).await,
            "find_symbols" => self.handle_find_symbols(arguments).await,
            "get_repo_map" => self.handle_get_repo_map(arguments).await,
            "search_files" => self.handle_search_files(arguments).await,
            "read_snippet" => self.handle_read_snippet(arguments).await,
            "index_codebase" => self.handle_index_codebase(arguments).await,
            "index_status" => self.handle_index_status(arguments).await,
            _ => CallToolResult::error(format!("Unknown tool: {}", name)),
        }
    }

    async fn handle_search_codebase(&self, args: &Value) -> CallToolResult {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) if !q.trim().is_empty() => q,
            _ => return CallToolResult::error("Missing required parameter: query"),
        };

        let top_k = args
            .get("top_k")
            .and_then(|v| v.as_u64())
            .map(|k| k as usize);

        match self.retriever.retrieve(query, top_k).await {
            Ok(chunks) => {
                let packed = self.context_packer.pack(&chunks);
                let json_output = json!({
                    "query": query,
                    "results_count": packed.blocks.len(),
                    "total_tokens_estimate": packed.total_estimated_tokens,
                    "context_block": packed.formatted_text,
                    "snippets": packed.blocks,
                });
                CallToolResult::text(serde_json::to_string_pretty(&json_output).unwrap())
            }
            Err(e) => CallToolResult::error(format!("Search failed: {}", e)),
        }
    }

    async fn handle_find_symbols(&self, args: &Value) -> CallToolResult {
        let symbol_query = match args.get("symbol").and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => s,
            _ => return CallToolResult::error("Missing required parameter: symbol"),
        };

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|l| l as usize)
            .unwrap_or(15);

        let store = self.engine.get_symbol_store();
        let results = store.read().await.search(symbol_query, limit);

        let json_output = json!({
            "query": symbol_query,
            "count": results.len(),
            "symbols": results.into_iter().map(|r| {
                json!({
                    "name": r.symbol.name,
                    "kind": r.symbol.kind,
                    "path": r.symbol.relative_path,
                    "signature": r.symbol.signature,
                    "scope": r.symbol.parent_scope,
                    "line_start": r.symbol.line_start,
                    "line_end": r.symbol.line_end,
                    "score": r.score,
                })
            }).collect::<Vec<_>>()
        });

        CallToolResult::text(serde_json::to_string_pretty(&json_output).unwrap())
    }

    async fn handle_get_repo_map(&self, args: &Value) -> CallToolResult {
        let max_tokens = args
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|t| t as usize)
            .unwrap_or(4000);

        let map = self.repo_map_gen.generate_map(max_tokens).await;
        CallToolResult::text(map)
    }

    async fn handle_search_files(&self, args: &Value) -> CallToolResult {
        let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p.to_lowercase(),
            None => return CallToolResult::error("Missing required parameter: pattern"),
        };

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|l| l as usize)
            .unwrap_or(20);

        let root = &self.engine.get_config().project_root;
        let all_files = self.engine.scan_workspace(root);

        let mut matches = Vec::new();
        for file in all_files {
            let rel = file
                .strip_prefix(root)
                .unwrap_or(&file)
                .to_string_lossy()
                .to_string();

            let rel_lower = rel.to_lowercase();
            let score = if rel_lower.contains(&pattern) {
                1.0f32
            } else {
                jaro_winkler(&pattern, &rel_lower) as f32
            };

            if score > 0.6 {
                matches.push((score, rel));
            }
        }

        matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        matches.truncate(limit);

        let json_output = json!({
            "pattern": pattern,
            "matches": matches.into_iter().map(|(score, path)| json!({ "path": path, "score": score })).collect::<Vec<_>>()
        });

        CallToolResult::text(serde_json::to_string_pretty(&json_output).unwrap())
    }

    async fn handle_read_snippet(&self, args: &Value) -> CallToolResult {
        let path_str = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return CallToolResult::error("Missing required parameter: path"),
        };

        let start_line = match args.get("start_line").and_then(|v| v.as_u64()) {
            Some(s) => s as usize,
            None => return CallToolResult::error("Missing required parameter: start_line"),
        };

        let end_line = match args.get("end_line").and_then(|v| v.as_u64()) {
            Some(e) => e as usize,
            None => return CallToolResult::error("Missing required parameter: end_line"),
        };

        let root = &self.engine.get_config().project_root;
        let file_path = if Path::new(path_str).is_absolute() {
            Path::new(path_str).to_path_buf()
        } else {
            root.join(path_str)
        };

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => return CallToolResult::error(format!("Failed to read file {}: {}", path_str, e)),
        };

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let s = start_line.saturating_sub(1);
        let e = end_line.min(total_lines);

        if s >= total_lines || s >= e {
            return CallToolResult::error(format!(
                "Invalid line range {}-{} for file with {} lines",
                start_line, end_line, total_lines
            ));
        }

        let mut out = String::new();
        for (i, line) in lines[s..e].iter().enumerate() {
            out.push_str(&format!("{:4} | {}\n", s + i + 1, line));
        }

        CallToolResult::text(out)
    }

    async fn handle_index_codebase(&self, _args: &Value) -> CallToolResult {
        match self.engine.index_all(false).await {
            Ok(stats) => {
                let json_output = json!({
                    "status": "success",
                    "stats": stats
                });
                CallToolResult::text(serde_json::to_string_pretty(&json_output).unwrap())
            }
            Err(e) => CallToolResult::error(format!("Indexing failed: {}", e)),
        }
    }

    async fn handle_index_status(&self, _args: &Value) -> CallToolResult {
        match self.engine.get_stats().await {
            Ok(stats) => {
                CallToolResult::text(serde_json::to_string_pretty(&stats).unwrap())
            }
            Err(e) => CallToolResult::error(format!("Failed to retrieve index status: {}", e)),
        }
    }
}
