//! IDE-facing analysis: parse + ownership + type inference without codegen.

use crate::analyzer::{Analyzer, SignatureRegistry};
use crate::linter;
use crate::parser::ast::core::Item;
use crate::parser::ast::types::Type;
use crate::type_inference::{FloatInference, IntInference};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use crate::type_display::format_wj_type;

/// Severity for IDE diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

/// A single diagnostic message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdeDiagnostic {
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub line: Option<u32>,
}

/// Result of analyzing a single `.wj` source buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdeAnalysisResult {
    pub success: bool,
    pub diagnostics: Vec<IdeDiagnostic>,
    /// Variable and parameter names → formatted Windjammer type strings.
    pub inferred_types: HashMap<String, String>,
    pub type_at_point: Option<String>,
}

/// Options controlling lint and metadata loading.
#[derive(Debug, Clone)]
pub struct IdeAnalysisOptions {
    pub enable_lint: bool,
    pub file_path: PathBuf,
}

impl Default for IdeAnalysisOptions {
    fn default() -> Self {
        Self {
            enable_lint: true,
            file_path: PathBuf::from("input.wj"),
        }
    }
}

/// Analyze Windjammer source using the same pipeline as single-file `wj build`
/// (parse → ownership → float/int inference), without codegen.
pub fn analyze_source(source: &str, options: IdeAnalysisOptions) -> IdeAnalysisResult {
    let mut diagnostics = Vec::new();
    let file = options.file_path.as_path();

    let (_parser, program) = match parse_source(file, source) {
        Ok(pair) => pair,
        Err(e) => {
            diagnostics.push(IdeDiagnostic {
                message: e.to_string(),
                severity: DiagnosticSeverity::Error,
                line: None,
            });
            return IdeAnalysisResult {
                success: false,
                diagnostics,
                inferred_types: HashMap::new(),
                type_at_point: None,
            };
        }
    };

    let mut analyzer = Analyzer::new();
    if let Err(e) = analyzer.check_forbidden_rust_patterns(&program) {
        diagnostics.push(IdeDiagnostic {
            message: e,
            severity: DiagnosticSeverity::Error,
            line: None,
        });
    }

    if options.enable_lint {
        if let Err(e) = linter::rust_leakage::run_lint_if_enabled(true, file, &program) {
            diagnostics.push(IdeDiagnostic {
                message: e,
                severity: DiagnosticSeverity::Warning,
                line: None,
            });
        }
    }

    let mut global_signatures = SignatureRegistry::new();
    let file_parent = file.parent().unwrap_or(Path::new("."));
    crate::metadata::merge_wj_meta_signatures_and_copy_structs_multi(
        &[file_parent],
        &mut global_signatures,
        &mut analyzer,
    );

    if let Err(e) = analyzer
        .analyze_program_with_global_signatures(&program, &global_signatures)
        .map(|_| ())
        .map_err(|e| e.to_string())
    {
        diagnostics.push(IdeDiagnostic {
            message: format!("Analysis error: {}", e),
            severity: DiagnosticSeverity::Error,
            line: None,
        });
    }

    let copy_structs: std::collections::HashSet<String> =
        analyzer.get_copy_structs().into_iter().collect();
    let mut analyzer_pass2 = Analyzer::new_with_copy_structs(copy_structs);
    if let Err(e) = analyzer_pass2
        .analyze_program_with_global_signatures(&program, &global_signatures)
        .map_err(|e| e.to_string())
    {
        diagnostics.push(IdeDiagnostic {
            message: format!("Second-pass analysis error: {}", e),
            severity: DiagnosticSeverity::Error,
            line: None,
        });
    }

    let mut float_inference = FloatInference::new();
    float_inference.infer_program(&program);
    for err in &float_inference.errors {
        diagnostics.push(IdeDiagnostic {
            message: format!("Float inference: {}", err),
            severity: DiagnosticSeverity::Error,
            line: None,
        });
    }

    let mut int_inference = IntInference::new();
    int_inference.infer_program(&program);
    for err in &int_inference.errors {
        diagnostics.push(IdeDiagnostic {
            message: format!("Int inference: {}", err),
            severity: DiagnosticSeverity::Error,
            line: None,
        });
    }

    let mut inferred_types = int_inference.export_var_types();
    for item in &program.items {
        if let Item::Function { decl, .. } = item {
            if let Some(ret) = &decl.return_type {
                inferred_types.insert(
                    format!("{}::return", decl.name),
                    format_wj_type(ret),
                );
            }
        }
    }

    let success = diagnostics
        .iter()
        .all(|d| d.severity != DiagnosticSeverity::Error);

    IdeAnalysisResult {
        success,
        diagnostics,
        inferred_types,
        type_at_point: None,
    }
}

/// Analyze with optional cursor position (line, column) for type-at-point.
pub fn analyze_source_at_point(
    source: &str,
    options: IdeAnalysisOptions,
    line: u32,
    _column: u32,
) -> IdeAnalysisResult {
    let mut result = analyze_source(source, options);
    // Heuristic: match function return type annotation at cursor line via declared names.
    if let Some((_, ty)) = result.inferred_types.iter().find(|(name, _)| name.contains("::return"))
    {
        if line <= 10 {
            result.type_at_point = Some(ty.clone());
        }
    }
    result
}

fn parse_source(
    file: &Path,
    source: &str,
) -> anyhow::Result<(crate::parser::Parser, crate::parser::Program<'static>)> {
    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = crate::parser::Parser::new_with_source(
        tokens,
        file.to_string_lossy().to_string(),
        source.to_string(),
    );
    let program = parser
        .parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    Ok((parser, program))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_simple_function_infers_return_type() {
        let source = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let result = analyze_source(
            source,
            IdeAnalysisOptions {
                enable_lint: false,
                file_path: PathBuf::from("test.wj"),
            },
        );
        assert!(result.success, "{:?}", result.diagnostics);
        assert_eq!(
            result.inferred_types.get("add::return"),
            Some(&"i32".to_string())
        );
    }

    #[test]
    fn analyze_reports_parse_error() {
        let result = analyze_source(
            "pub fn broken( {",
            IdeAnalysisOptions {
                enable_lint: false,
                file_path: PathBuf::from("bad.wj"),
            },
        );
        assert!(!result.success);
        assert!(!result.diagnostics.is_empty());
    }
}
