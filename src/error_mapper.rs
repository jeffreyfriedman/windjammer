/// Error Mapper: Translates Rust compiler errors back to Windjammer source locations
///
/// This module intercepts rustc JSON output, maps errors using source maps,
/// and provides a world-class error experience for Windjammer developers.
use crate::source_map::{Location, SourceMap};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// CARGO JSON OUTPUT STRUCTURES
// ============================================================================

/// Cargo message wrapper (from --message-format=json)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CargoMessage {
    /// Message reason (e.g., "compiler-message", "compiler-artifact")
    pub reason: String,
    /// Compiler diagnostic (if reason is "compiler-message")
    pub message: Option<RustcDiagnostic>,
}

// ============================================================================
// RUSTC JSON OUTPUT STRUCTURES
// ============================================================================

/// Rustc diagnostic message (from --error-format=json)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RustcDiagnostic {
    /// Error message text
    pub message: String,
    /// Severity level (error, warning, note, help)
    pub level: String,
    /// Primary code span (file, line, column)
    pub spans: Vec<RustcSpan>,
    /// Error code (e.g., "E0308")
    pub code: Option<RustcCode>,
    /// Child diagnostics (notes, help messages)
    pub children: Vec<RustcDiagnostic>,
    /// Rendered text (for fallback display)
    pub rendered: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RustcSpan {
    /// File path
    pub file_name: String,
    /// Line number (1-indexed)
    pub line_start: usize,
    pub line_end: usize,
    /// Column number (1-indexed)
    pub column_start: usize,
    pub column_end: usize,
    /// Whether this is the primary span
    pub is_primary: bool,
    /// Label text for this span
    pub label: Option<String>,
    /// Suggested replacement (for fix suggestions)
    pub suggested_replacement: Option<String>,
    /// Text content of the span (lines of source code)
    pub text: Option<Vec<RustcSpanText>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RustcSpanText {
    /// Text content
    pub text: String,
    /// Whether this line is highlighted
    pub highlight_start: usize,
    pub highlight_end: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RustcCode {
    /// Error code (e.g., "E0308")
    pub code: String,
    /// Explanation text
    pub explanation: Option<String>,
}

// ============================================================================
// WINDJAMMER DIAGNOSTIC STRUCTURES
// ============================================================================

/// Windjammer diagnostic message (mapped from Rust)
#[derive(Debug, Clone)]
pub struct WindjammerDiagnostic {
    /// Error message (translated to Windjammer terminology)
    pub message: String,
    /// Severity level
    pub level: DiagnosticLevel,
    /// Primary location in Windjammer source
    pub location: Location,
    /// Additional spans (for multi-location errors)
    pub spans: Vec<DiagnosticSpan>,
    /// Error code (if applicable)
    pub code: Option<String>,
    /// Help messages
    pub help: Vec<String>,
    /// Notes
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
    Help,
}

#[derive(Debug, Clone)]
pub struct DiagnosticSpan {
    /// Location in Windjammer source
    pub location: Location,
    /// Label for this span
    pub label: Option<String>,
    /// Whether this is the primary span
    pub is_primary: bool,
}

// ============================================================================
// ERROR MAPPER
// ============================================================================

pub struct ErrorMapper {
    source_map: SourceMap,
}

impl ErrorMapper {
    /// Create a new error mapper with the given source map
    pub fn new(source_map: SourceMap) -> Self {
        Self { source_map }
    }

    /// Parse rustc JSON output and map errors to Windjammer source
    pub fn map_rustc_output(&self, json_output: &str) -> Vec<WindjammerDiagnostic> {
        let mut diagnostics = Vec::new();

        for line in json_output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Parse JSON diagnostic
            if let Ok(rustc_diag) = serde_json::from_str::<RustcDiagnostic>(line) {
                // Only process errors and warnings (skip compiler messages)
                if rustc_diag.level == "error" || rustc_diag.level == "warning" {
                    if let Some(wj_diag) = self.map_diagnostic(&rustc_diag) {
                        diagnostics.push(wj_diag);
                    }
                }
            }
        }

        diagnostics
    }

    /// Map a single rustc diagnostic to Windjammer
    fn map_diagnostic(&self, rustc_diag: &RustcDiagnostic) -> Option<WindjammerDiagnostic> {
        // Find the primary span
        let primary_span = rustc_diag.spans.iter().find(|s| s.is_primary)?;

        // Map Rust location to Windjammer location
        let rust_location = Location {
            file: PathBuf::from(&primary_span.file_name),
            line: primary_span.line_start,
            column: primary_span.column_start,
        };

        let wj_location = self.source_map.map_rust_to_windjammer(&rust_location)?;

        // Translate error message
        let message = self.translate_message(&rustc_diag.message);

        // Map additional spans
        let spans = rustc_diag
            .spans
            .iter()
            .filter_map(|span| self.map_span(span))
            .collect();

        // Extract help and notes from children
        let mut help = Vec::new();
        let mut notes = Vec::new();

        for child in &rustc_diag.children {
            match child.level.as_str() {
                "help" => help.push(child.message.clone()),
                "note" => notes.push(child.message.clone()),
                _ => {}
            }
        }

        Some(WindjammerDiagnostic {
            message,
            level: match rustc_diag.level.as_str() {
                "error" => DiagnosticLevel::Error,
                "warning" => DiagnosticLevel::Warning,
                "note" => DiagnosticLevel::Note,
                "help" => DiagnosticLevel::Help,
                _ => DiagnosticLevel::Error,
            },
            location: wj_location,
            spans,
            code: rustc_diag.code.as_ref().map(|c| c.code.clone()),
            help,
            notes,
        })
    }

    /// Map a rustc span to a Windjammer span
    fn map_span(&self, span: &RustcSpan) -> Option<DiagnosticSpan> {
        let rust_location = Location {
            file: PathBuf::from(&span.file_name),
            line: span.line_start,
            column: span.column_start,
        };

        let wj_location = self.source_map.map_rust_to_windjammer(&rust_location)?;

        Some(DiagnosticSpan {
            location: wj_location,
            label: span.label.clone(),
            is_primary: span.is_primary,
        })
    }

    /// Translate Rust error messages to Windjammer terminology
    fn translate_message(&self, rust_msg: &str) -> String {
        // TODO: Implement comprehensive message translation
        // For now, just return the original message
        // Phase 3 will add sophisticated translation patterns

        rust_msg.to_string()
    }
}

// ============================================================================
// PRETTY PRINTING
// ============================================================================

impl WindjammerDiagnostic {
    /// Format this diagnostic for display (Rust-style pretty printing)
    pub fn format(&self) -> String {
        let mut output = String::new();

        // Level and message
        let level_str = match self.level {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Note => "note",
            DiagnosticLevel::Help => "help",
        };

        if let Some(code) = &self.code {
            output.push_str(&format!("{}[{}]: {}\n", level_str, code, self.message));
        } else {
            output.push_str(&format!("{}: {}\n", level_str, self.message));
        }

        // Location
        output.push_str(&format!(
            "  --> {}:{}:{}\n",
            self.location.file.display(),
            self.location.line,
            self.location.column
        ));

        // TODO: Add source code snippet display (Phase 3)

        // Help messages
        for help_msg in &self.help {
            output.push_str(&format!("  = help: {}\n", help_msg));
        }

        // Notes
        for note in &self.notes {
            output.push_str(&format!("  = note: {}\n", note));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rustc_json() {
        let json = r#"{"message":"mismatched types","level":"error","spans":[{"file_name":"test.rs","line_start":10,"line_end":10,"column_start":5,"column_end":10,"is_primary":true,"label":"expected i32, found &str"}],"code":{"code":"E0308"},"children":[],"rendered":null}"#;

        let diag: RustcDiagnostic = serde_json::from_str(json).unwrap();
        assert_eq!(diag.message, "mismatched types");
        assert_eq!(diag.level, "error");
        assert_eq!(diag.spans.len(), 1);
        assert_eq!(diag.spans[0].line_start, 10);
    }

    #[test]
    fn test_diagnostic_format() {
        let diag = WindjammerDiagnostic {
            message: "Type mismatch".to_string(),
            level: DiagnosticLevel::Error,
            location: Location {
                file: PathBuf::from("test.wj"),
                line: 10,
                column: 5,
            },
            spans: vec![],
            code: Some("E0308".to_string()),
            help: vec!["Try using .parse()".to_string()],
            notes: vec![],
        };

        let formatted = diag.format();
        assert!(formatted.contains("error[E0308]"));
        assert!(formatted.contains("test.wj:10:5"));
        assert!(formatted.contains("help: Try using .parse()"));
    }
}
