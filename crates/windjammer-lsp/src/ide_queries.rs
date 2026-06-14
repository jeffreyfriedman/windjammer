//! IDE analysis helpers for LSP database (calls `windjammer::ide_analysis`).

use crate::database::{SourceFile, Symbol, SymbolKind};
use tower_lsp::lsp_types::{
    InlayHint, InlayHintKind, InlayHintLabel, InlayHintTooltip, Position,
};
use windjammer::ide_analysis::{
    analyze_source, analyze_source_at_point, DiagnosticSeverity, IdeAnalysisOptions,
    IdeDiagnostic, IdeOwnershipHint,
};

/// IDE analysis snapshot for a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdeAnalysisSnapshot {
    pub success: bool,
    pub diagnostics: Vec<IdeDiagnostic>,
    pub inferred_type_pairs: Vec<(String, String)>,
    pub ownership_hints: Vec<IdeOwnershipHint>,
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
        ownership_hints: result.ownership_hints,
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

/// Build LSP inlay hints from ide_analysis output and Salsa symbol positions.
pub fn to_inlay_hints(
    snapshot: &IdeAnalysisSnapshot,
    symbols: &[Symbol],
    source: &str,
) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    let inferred = snapshot.inferred_types();

    for symbol in symbols {
        if symbol.kind != SymbolKind::Function {
            continue;
        }
        let return_key = format!("{}::return", symbol.name);
        if let Some(return_type) = inferred.get(&return_key) {
            if let Some(range) = &symbol.range {
                hints.push(InlayHint {
                    position: Position {
                        line: range.end_line,
                        character: range.end_character,
                    },
                    label: InlayHintLabel::String(format!(": {}", return_type)),
                    kind: Some(InlayHintKind::TYPE),
                    text_edits: None,
                    tooltip: Some(InlayHintTooltip::String(format!(
                        "Return type of {}",
                        symbol.name
                    ))),
                    padding_left: Some(false),
                    padding_right: Some(false),
                    data: None,
                });
            }
        }
    }

    for hint in &snapshot.ownership_hints {
        if let Some(position) = parameter_hint_position(source, &hint.function_name, &hint.parameter_name)
        {
            hints.push(InlayHint {
                position,
                label: InlayHintLabel::String(format!(
                    "{}: {}",
                    hint.parameter_name,
                    hint.mode.inlay_suffix()
                )),
                kind: Some(InlayHintKind::PARAMETER),
                text_edits: None,
                tooltip: Some(InlayHintTooltip::String(format!(
                    "Inferred ownership: {}",
                    hint.mode.label()
                ))),
                padding_left: Some(false),
                padding_right: Some(true),
                data: None,
            });
        }
    }

    hints
}

fn parameter_hint_position(source: &str, function_name: &str, parameter_name: &str) -> Option<Position> {
    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.contains("fn ") || !trimmed.contains(function_name) {
            continue;
        }
        if !(trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.contains(&format!(" fn {}", function_name))
            || trimmed.contains(&format!(" fn {function_name}(")))
        {
            continue;
        }
        if let Some(param_start) = find_parameter_token(line, parameter_name) {
            return Some(Position {
                line: line_idx as u32,
                character: (param_start + parameter_name.len()) as u32,
            });
        }
    }
    None
}

fn find_parameter_token(line: &str, parameter_name: &str) -> Option<usize> {
    let mut search_from = 0;
    while let Some(rel) = line[search_from..].find(parameter_name) {
        let start = search_from + rel;
        let before_ok = start == 0 || !line.as_bytes()[start - 1].is_ascii_alphanumeric();
        let end = start + parameter_name.len();
        let after_ok = end >= line.len() || !line.as_bytes()[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return Some(start);
        }
        search_from = start + parameter_name.len();
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_position_finds_fn_param() {
        let source = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
        let pos = parameter_hint_position(source, "add", "a").expect("param a");
        assert_eq!(pos.line, 0);
        assert!(pos.character > 0);
    }

    #[test]
    fn to_inlay_hints_includes_return_type() {
        let snapshot = IdeAnalysisSnapshot {
            success: true,
            diagnostics: vec![],
            inferred_type_pairs: vec![("add::return".to_string(), "i32".to_string())],
            ownership_hints: vec![],
        };
        let symbols = vec![Symbol {
            name: "add".to_string(),
            kind: SymbolKind::Function,
            line: 0,
            character: 0,
            range: Some(crate::database::SymbolRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 30,
            }),
            name_range: None,
            type_info: Some("i32".to_string()),
            doc: None,
        }];
        let hints = to_inlay_hints(&snapshot, &symbols, "pub fn add(a: i32, b: i32) -> i32 {}");
        assert!(!hints.is_empty());
    }
}
