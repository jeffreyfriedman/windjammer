//! IDE analysis helpers for LSP database (calls `windjammer::ide_analysis`).

use crate::database::SourceFile;
use windjammer::ide_analysis::{
    analyze_source, analyze_source_at_point, DiagnosticSeverity, IdeAnalysisOptions,
    IdeDiagnostic,
};

/// IDE analysis snapshot for a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdeAnalysisSnapshot {
    pub success: bool,
    pub diagnostics: Vec<IdeDiagnostic>,
    pub inferred_type_pairs: Vec<(String, String)>,
}

impl IdeAnalysisSnapshot {
    pub fn inferred_types(&self) -> std::collections::HashMap<String, String> {
        self.inferred_type_pairs.iter().cloned().collect()
    }
}

pub fn run_ide_analysis(file: &SourceFile, db: &dyn salsa::Database) -> IdeAnalysisSnapshot {
    let uri = file.uri(db);
    let text = file.text(db);
    let path = uri.to_file_path().unwrap_or_else(|_| std::path::PathBuf::from("input.wj"));

    let result = analyze_source(
        text,
        IdeAnalysisOptions {
            enable_lint: true,
            file_path: path,
        },
    );

    IdeAnalysisSnapshot {
        success: result.success,
        diagnostics: result.diagnostics,
        inferred_type_pairs: result.inferred_types.into_iter().collect(),
    }
}

pub fn run_type_at_point(
    file: &SourceFile,
    db: &dyn salsa::Database,
    line: u32,
    column: u32,
) -> Option<String> {
    let uri = file.uri(db);
    let text = file.text(db);
    let path = uri.to_file_path().unwrap_or_else(|_| std::path::PathBuf::from("input.wj"));

    analyze_source_at_point(
        text,
        IdeAnalysisOptions {
            enable_lint: false,
            file_path: path,
        },
        line,
        column,
    )
    .type_at_point
}

/// Convert IDE diagnostics to LSP `Diagnostic` values.
pub fn to_lsp_diagnostics(diagnostics: &[IdeDiagnostic]) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    use tower_lsp::lsp_types::{Diagnostic, Position, Range};
    use tower_lsp::lsp_types::DiagnosticSeverity as LspSeverity;

    diagnostics
        .iter()
        .map(|d| {
            let severity = match d.severity {
                DiagnosticSeverity::Error => Some(LspSeverity::ERROR),
                DiagnosticSeverity::Warning => Some(LspSeverity::WARNING),
                DiagnosticSeverity::Info => Some(LspSeverity::INFORMATION),
            };
            let line = d.line.unwrap_or(1).saturating_sub(1);
            Diagnostic {
                range: Range {
                    start: Position {
                        line,
                        character: 0,
                    },
                    end: Position {
                        line,
                        character: u32::MAX,
                    },
                },
                severity,
                code: None,
                code_description: None,
                source: Some("windjammer".to_string()),
                message: d.message.clone(),
                related_information: None,
                tags: None,
                data: None,
            }
        })
        .collect()
}

/// Convert IDE diagnostics to LSP-style message strings.
pub fn format_diagnostic(d: &IdeDiagnostic) -> String {
    let prefix = match d.severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
    };
    match d.line {
        Some(line) => format!("{} [line {}]: {}", prefix, line, d.message),
        None => format!("{}: {}", prefix, d.message),
    }
}
