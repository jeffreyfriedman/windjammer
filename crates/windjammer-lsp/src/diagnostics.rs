use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString,
    Position, Range, Url,
};
use tower_lsp::Client;
use windjammer::error_mapper::{DiagnosticLevel, WindjammerDiagnostic};

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

    /// Publish Windjammer diagnostics (with error codes and rich information)
    pub async fn publish_windjammer(&self, uri: &Url, wj_diagnostics: Vec<WindjammerDiagnostic>) {
        let diagnostics: Vec<Diagnostic> = wj_diagnostics
            .into_iter()
            .map(|wj_diag| self.convert_windjammer_diagnostic(wj_diag))
            .collect();

        self.publish(uri, diagnostics).await;
    }

    /// Convert a Windjammer diagnostic to an LSP diagnostic
    fn convert_windjammer_diagnostic(&self, wj_diag: WindjammerDiagnostic) -> Diagnostic {
        // Convert severity
        let severity = match wj_diag.level {
            DiagnosticLevel::Error => DiagnosticSeverity::ERROR,
            DiagnosticLevel::Warning => DiagnosticSeverity::WARNING,
            DiagnosticLevel::Note => DiagnosticSeverity::INFORMATION,
            DiagnosticLevel::Help => DiagnosticSeverity::HINT,
        };

        // Create range (LSP uses 0-indexed positions)
        let range = Range {
            start: Position {
                line: (wj_diag.location.line.saturating_sub(1)) as u32,
                character: (wj_diag.location.column.saturating_sub(1)) as u32,
            },
            end: Position {
                line: (wj_diag.location.line.saturating_sub(1)) as u32,
                character: (wj_diag.location.column + 10) as u32, // Approximate end
            },
        };

        // Build message with help and notes
        let mut message = wj_diag.message.clone();

        if !wj_diag.help.is_empty() {
            message.push_str("\n\nHelp:");
            for help in &wj_diag.help {
                message.push_str(&format!("\n  • {}", help));
            }
        }

        if !wj_diag.notes.is_empty() {
            message.push_str("\n\nNotes:");
            for note in &wj_diag.notes {
                message.push_str(&format!("\n  • {}", note));
            }
        }

        // Add contextual help if available
        if let Some(contextual_help) = self.get_contextual_help(&wj_diag) {
            message.push_str(&format!("\n\n💡 Suggestion: {}", contextual_help));
        }

        // Add "wj explain" hint for Windjammer codes
        if let Some(ref code) = wj_diag.code {
            if code.starts_with("WJ") {
                message.push_str(&format!(
                    "\n\n💡 Run 'wj explain {}' for more details",
                    code
                ));
            }
        }

        // Convert related information
        let related_information = if !wj_diag.spans.is_empty() {
            Some(
                wj_diag
                    .spans
                    .iter()
                    .filter_map(|span| {
                        let uri = Url::from_file_path(&span.location.file).ok()?;
                        Some(DiagnosticRelatedInformation {
                            location: Location {
                                uri,
                                range: Range {
                                    start: Position {
                                        line: (span.location.line.saturating_sub(1)) as u32,
                                        character: (span.location.column.saturating_sub(1)) as u32,
                                    },
                                    end: Position {
                                        line: (span.location.line.saturating_sub(1)) as u32,
                                        character: (span.location.column + 10) as u32,
                                    },
                                },
                            },
                            message: span.label.clone().unwrap_or_default(),
                        })
                    })
                    .collect(),
            )
        } else {
            None
        };

        Diagnostic {
            range,
            severity: Some(severity),
            code: wj_diag.code.map(NumberOrString::String),
            code_description: None,
            source: Some("windjammer".to_string()),
            message,
            related_information,
            tags: None,
            data: None,
        }
    }

    /// Get contextual help for a diagnostic
    fn get_contextual_help(&self, wj_diag: &WindjammerDiagnostic) -> Option<String> {
        let msg = &wj_diag.message.to_lowercase();

        // Type conversion hints
        if msg.contains("expected int") && msg.contains("found string") {
            return Some("Use .parse() to convert string to int".to_string());
        }
        if msg.contains("expected string") && msg.contains("found") {
            return Some("Use .to_string() to convert to string".to_string());
        }

        // Mutability hints
        if msg.contains("cannot") && msg.contains("mutable") {
            return Some("Declare the variable as mutable: let mut x = ...".to_string());
        }

        // Ownership hints (though auto-clone should handle most cases)
        if msg.contains("moved") || msg.contains("ownership") {
            return Some(
                "The auto-clone system should handle this. If you see this, please report it!"
                    .to_string(),
            );
        }

        None
    }

    /// Clear all diagnostics for a file
    pub async fn clear(&self, uri: &Url) {
        tracing::debug!("Clearing diagnostics for {}", uri);

        self.client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;
    }
}
