use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::index::IndexEngine;
use crate::mcp::McpServer;
use crate::retrieval::{ContextPacker, HybridRetriever, RepoMapGenerator};

#[derive(Parser)]
#[command(
    name = "codebase-rag",
    version = "2.0.0",
    about = "AST-aware hybrid Codebase RAG & MCP Server for modern AI coding agents",
    long_about = "codebase-rag-2.0 provides lightning-fast AST-aware code retrieval combining Tantivy BM25, embedded vector search, and Tree-Sitter symbol resolution via the Model Context Protocol (MCP)."
)]
pub struct Cli {
    #[arg(short, long, global = true, help = "Path to project root directory")]
    pub project: Option<PathBuf>,

    #[arg(long, help = "Run directly in MCP stdio server mode (default if no subcommand is passed)")]
    pub mcp: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Index the codebase files, AST symbols, and vector embeddings")]
    Index {
        #[arg(help = "Path to workspace to index")]
        path: Option<PathBuf>,
    },

    #[command(about = "Perform hybrid search across the codebase")]
    Search {
        #[arg(help = "Query string")]
        query: String,

        #[arg(short = 'k', long, default_value_t = 5, help = "Number of results to return")]
        top_k: usize,
    },

    #[command(about = "Search AST symbol definitions and hierarchy")]
    Symbols {
        #[arg(help = "Symbol identifier to search for")]
        symbol: String,

        #[arg(short, long, default_value_t = 15, help = "Maximum results")]
        limit: usize,
    },

    #[command(about = "Generate a compressed structural skeleton repo map")]
    Map {
        #[arg(short, long, default_value_t = 4000, help = "Maximum token budget")]
        tokens: usize,
    },

    #[command(about = "Display indexing and storage statistics")]
    Status,

    #[command(about = "Start the Model Context Protocol (MCP) server")]
    Serve {
        #[arg(long, default_value_t = true, help = "Serve over stdio JSON-RPC")]
        stdio: bool,
    },
}

pub async fn run_cli(cli: Cli) -> Result<()> {
    let project_root = cli
        .project
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let config = AppConfig::new(&project_root);
    let engine = Arc::new(IndexEngine::new(config)?);

    // If --mcp or no subcommand is passed, start the MCP stdio server
    if cli.mcp || cli.command.is_none() {
        let server = McpServer::new(engine);
        server.run_stdio().await?;
        return Ok(());
    }

    match cli.command.unwrap() {
        Commands::Index { path } => {
            let target_root = path.unwrap_or(project_root);
            println!("⚡ Indexing workspace: {}", target_root.display());
            let stats = engine.index_all(true).await?;
            println!("\n✓ Indexing complete!");
            println!("  • Files: {}", stats.total_files);
            println!("  • AST Chunks: {}", stats.total_chunks);
            println!("  • Symbols: {}", stats.total_symbols);
            println!("  • Vectors: {}", stats.total_vectors);
            println!("  • Storage: {}", stats.storage_dir);
        }
        Commands::Search { query, top_k } => {
            let tantivy = engine.get_tantivy_store();
            let vector = engine.get_vector_store();
            let symbol = engine.get_symbol_store();
            let embedding = engine.get_embedding_engine();
            let ret_config = engine.get_config().retrieval.clone();

            let retriever = HybridRetriever::new(tantivy, vector, symbol, embedding, ret_config.clone());
            let chunks = retriever.retrieve(&query, Some(top_k)).await?;

            let packer = ContextPacker::new(ret_config.max_context_tokens);
            let packed = packer.pack(&chunks);

            println!("\n🔍 Hybrid Search Results for: \"{}\"", query);
            println!("Found {} relevant code snippets (~{} tokens):\n", packed.blocks.len(), packed.total_estimated_tokens);
            println!("{}", packed.formatted_text);
        }
        Commands::Symbols { symbol, limit } => {
            let store = engine.get_symbol_store();
            let results = store.read().await.search(&symbol, limit);

            println!("\n🔎 AST Symbols matching: \"{}\"", symbol);
            if results.is_empty() {
                println!("No symbols found.");
            } else {
                for (i, res) in results.iter().enumerate() {
                    let sym = &res.symbol;
                    let scope = sym.parent_scope.as_deref().map(|s| format!("{}::", s)).unwrap_or_default();
                    println!(
                        "{:2}. [{}] {}{}: {}:L{}-L{} (score: {:.2})",
                        i + 1,
                        sym.kind,
                        scope,
                        sym.name,
                        sym.relative_path,
                        sym.line_start,
                        sym.line_end,
                        res.score
                    );
                    println!("    `{}`", sym.signature);
                }
            }
        }
        Commands::Map { tokens } => {
            let symbol_store = engine.get_symbol_store();
            let generator = RepoMapGenerator::new(symbol_store);
            let map = generator.generate_map(tokens).await;
            println!("{}", map);
        }
        Commands::Status => {
            let stats = engine.get_stats().await?;
            println!("\n📊 codebase-rag-2.0 Status");
            println!("  • Project Root: {}", project_root.display());
            println!("  • Storage Dir: {}", stats.storage_dir);
            println!("  • Total Files: {}", stats.total_files);
            println!("  • Total Chunks: {}", stats.total_chunks);
            println!("  • Total Symbols: {}", stats.total_symbols);
            println!("  • Total Vectors: {}", stats.total_vectors);
            println!("  • Embedding Provider: {}", stats.embedding_provider);
            println!("  • Embedding Model: {}", stats.embedding_model);
        }
        Commands::Serve { stdio } => {
            if stdio {
                let server = McpServer::new(engine);
                server.run_stdio().await?;
            }
        }
    }

    Ok(())
}
