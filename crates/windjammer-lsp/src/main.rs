use tower_lsp::{LspService, Server};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod analysis;
mod diagnostics;
mod server;

use server::WindjammerLanguageServer;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting Windjammer Language Server");

    // Set up stdin/stdout for LSP communication
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Create the LSP service
    let (service, socket) = LspService::new(|client| WindjammerLanguageServer::new(client));

    // Run the server
    Server::new(stdin, stdout, socket).serve(service).await;

    tracing::info!("Windjammer Language Server shutting down");
}
