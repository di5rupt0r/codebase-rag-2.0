# codebase-rag-2.0 Agent Integration Guide

`codebase-rag-2.0` is a universal Model Context Protocol (MCP) server designed to supercharge any AI agent with modern hybrid codebase indexing and retrieval (similar to Cursor and Windsurf).

## MCP Server Configuration

### 1. Claude Desktop / Claude Code

Add to `~/.config/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "codebase-rag": {
      "command": "/home/gabrielsb/codebase-rag-2.0/target/release/codebase-rag",
      "args": ["--mcp", "--project", "/path/to/your/project"],
      "env": {
        "GEMINI_API_KEY": "YOUR_GEMINI_API_KEY"
      }
    }
  }
}
```

### 2. Antigravity / Gemini

Add to your MCP configuration (`~/.gemini/antigravity/mcp/codebase-rag.json` or `mcp_servers` configuration):

```json
{
  "command": "/home/gabrielsb/codebase-rag-2.0/target/release/codebase-rag",
  "args": ["--mcp"]
}
```

### 3. Cursor / Windsurf

Add to MCP tools settings:

```json
{
  "mcpServers": {
    "codebase-rag": {
      "command": "/home/gabrielsb/codebase-rag-2.0/target/release/codebase-rag",
      "args": ["--mcp"],
      "env": {
        "CODEBASE_RAG_PROVIDER": "gemini",
        "GEMINI_API_KEY": "YOUR_GEMINI_API_KEY"
      }
    }
  }
}
```

### 4. Cline / Continue / OpenCode / Hermes

Configure MCP Stdio server pointing to the `codebase-rag` binary.

---

## Exposed MCP Tools

| Tool | Parameters | Description |
|---|---|---|
| `search_codebase` | `query: str`, `top_k: int?` | Multi-channel Hybrid Search (Tantivy BM25 + Vector + AST Symbols fused via RRF) returning relevant code snippets. |
| `find_symbols` | `symbol: str`, `limit: int?` | AST Symbol table lookup for classes, functions, structs, interfaces, methods across all supported languages. |
| `get_repo_map` | `max_tokens: int?` | Generates a compressed structural skeleton outline of the workspace. |
| `search_files` | `pattern: str`, `limit: int?` | Fuzzy path search ignoring gitignored files. |
| `read_snippet` | `path: str`, `start_line: int`, `end_line: int` | Precise line-slice extraction with line numbers. |
| `index_codebase` | *(none)* | Triggers full/incremental indexing. |
| `index_status` | *(none)* | Returns stats on indexed files, chunks, symbols, and storage. |
