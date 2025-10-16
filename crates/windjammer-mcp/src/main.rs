//! Windjammer MCP server binary

use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use windjammer_mcp::McpServer;

#[derive(Parser)]
#[command(name = "windjammer-mcp")]
#[command(about = "Model Context Protocol server for Windjammer", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the server with stdio transport (default)
    Stdio,

    /// Display server information
    Info,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let log_level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("windjammer_mcp={}", log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    match cli.command {
        Some(Commands::Stdio) | None => {
            // Default: run with stdio
            let server = McpServer::new().await?;
            server.run_stdio().await?;
        }
        Some(Commands::Info) => {
            println!("Windjammer MCP Server");
            println!("Version: {}", windjammer_mcp::SERVER_VERSION);
            println!("Protocol: {}", windjammer_mcp::MCP_VERSION);
            println!("\nThis server enables AI assistants to understand,");
            println!("analyze, and generate Windjammer code.");
        }
    }

    Ok(())
}
