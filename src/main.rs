use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use codebase_rag::cli::{run_cli, Cli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing logging to stderr so stdout is reserved for MCP JSON-RPC
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let cli = Cli::parse();
    run_cli(cli).await
}
