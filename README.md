# codebase-rag-2.0

> High-performance, AST-aware, hybrid Codebase RAG & MCP Server for modern AI coding agents (Cursor, Windsurf, Claude Code, Antigravity, OpenCode, Hermes, Cline).

## Overview

`codebase-rag-2.0` transforms how AI agents interact with complex codebases. Instead of legacy naive text chunking or slow vector-only scans, `codebase-rag-2.0` combines:

1. **Multi-Language AST Parsing**: Deep syntax analysis via Tree-Sitter (Rust, Python, TypeScript/JavaScript, Go, Java, C/C++) extracting structural symbols (functions, structs, classes, methods, traits, interfaces).
2. **Multi-Channel Hybrid Search**: Tantivy BM25 full-text + embedded Vector retrieval + exact AST symbol matching.
3. **Reciprocal Rank Fusion (RRF)**: Fusing lexical, semantic, and structural signals with token-budget packing and deduplication.
4. **Repository Skeleton & Map**: Compressed structural symbol graph and outline for contextual navigation.
5. **Incremental Git-Aware Indexing**: Fast file hashing and git diff awareness to instantly index large repositories.
6. **Unified MCP & CLI**: Model Context Protocol (MCP) server over standard IO (stdio) and CLI tooling (`index`, `search`, `symbols`, `map`, `serve`).
7. **API-First Embedding Client**: Asynchronous embedding provider support for Gemini, OpenAI, Voyage AI, and Ollama.

## Architecture

```
                               ┌─────────────────────────────┐
                               │       AI Coding Agent       │
                               │ (Cursor/Windsurf/Claude/AGY)│
                               └──────────────┬──────────────┘
                                              │ MCP (JSON-RPC)
                                              ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                             codebase-rag-2.0                               │
│                                                                            │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                            MCP Tools Layer                           │  │
│  │  search_codebase | find_symbols | get_repo_map | search_files | ...  │  │
│  └──────────────────────────────────┬───────────────────────────────────┘  │
│                                     │                                      │
│  ┌──────────────────────────────────▼───────────────────────────────────┐  │
│  │                     RRF Fusion & Context Packer                      │  │
│  └─────────────┬────────────────────┬────────────────────┬──────────────┘  │
│                │                    │                    │                 │
│  ┌─────────────▼───────┐ ┌──────────▼──────────┐ ┌───────▼──────────────┐  │
│  │     Tantivy BM25    │ │     Vector Store    │ │    Symbol Index      │  │
│  │   (Lexical Search)  │ │ (Semantic Embeddings│ │  (AST Symbol Graph)  │  │
│  └─────────────▲───────┘ └──────────▲──────────┘ └───────▲──────────────┘  │
│                │                    │                    │                 │
│  ┌─────────────┴────────────────────┴────────────────────┴──────────────┐  │
│  │                     AST Parser & Chunking Engine                     │  │
│  │               (Tree-Sitter / Language Grammars / Hasher)             │  │
│  └──────────────────────────────────▲───────────────────────────────────┘  │
│                                     │                                      │
│                                Codebase Files                              │
└────────────────────────────────────────────────────────────────────────────┘
```

## Quick Start

### Build

```bash
cargo build --release
```

### Run as MCP Server

```bash
./target/release/codebase-rag --mcp
```

### CLI Commands

```bash
# Index a workspace
codebase-rag index /path/to/project

# Hybrid search
codebase-rag search "authentication token middleware"

# Find symbols
codebase-rag symbols "UserProfile"

# Generate compressed repo map
codebase-rag map /path/to/project
```

## License

MIT OR Apache-2.0
