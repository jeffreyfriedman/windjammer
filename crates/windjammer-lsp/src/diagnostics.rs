use tower_lsp::lsp_types::{Diagnostic, Url};
use tower_lsp::Client;

/// Diagnostics engine for publishing errors and warnings to the client
pub struct DiagnosticsEngine {
    client: Client,
}

impl DiagnosticsEngine {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Publish diagnostics for a file
    pub async fn publish(&self, uri: &Url, diagnostics: Vec<Diagnostic>) {
        tracing::debug!("Publishing {} diagnostics for {}", diagnostics.len(), uri);

        self.client
            .publish_diagnostics(
                uri.clone(),
                diagnostics,
                None, // version
            )
            .await;
    }

    /// Clear all diagnostics for a file
    pub async fn clear(&self, uri: &Url) {
        tracing::debug!("Clearing diagnostics for {}", uri);

        self.client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;
    }
}
