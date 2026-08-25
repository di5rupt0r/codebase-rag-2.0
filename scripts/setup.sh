#!/usr/bin/env bash
set -e

echo "Building codebase-rag-2.0 in release mode..."
cargo build --release

BIN_PATH="$(pwd)/target/release/codebase-rag"

echo "✓ codebase-rag-2.0 built successfully at: $BIN_PATH"
echo ""
echo "To test MCP server:"
echo "  $BIN_PATH --mcp"
echo ""
echo "To index current workspace:"
echo "  $BIN_PATH index ."
echo ""
echo "To search:"
echo "  $BIN_PATH search \"query\""
