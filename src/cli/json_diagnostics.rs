//! Structured JSON diagnostic output (WJ-TOOL-01 Layer 1).
//!
//! When `--json` is passed to `wj build` or `wj check`, all compiler
//! diagnostics are emitted as structured JSON on stdout. This enables
//! AI agents and other tools to parse compiler output programmatically.

use serde::Serialize;

/// Schema version for the JSON diagnostic output.
pub const SCHEMA_VERSION: u32 = 1;

/// Top-level JSON output for a compilation run.
#[derive(Debug, Serialize)]
pub struct JsonCompilationOutput {
    pub schema_version: u32,
    pub windjammer_version: String,
    pub success: bool,
    pub diagnostics: Vec<JsonDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects_summary: Option<EffectsSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taint_summary: Option<TaintSummary>,
}

/// A single structured diagnostic.
#[derive(Debug, Serialize)]
pub struct JsonDiagnostic {
    pub code: Option<String>,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<DiagnosticSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maturity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair_hint: Option<RepairHint>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Source location span.
#[derive(Debug, Serialize)]
pub struct DiagnosticSpan {
    pub file: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

/// Machine-actionable repair suggestion.
#[derive(Debug, Serialize)]
pub struct RepairHint {
    pub kind: RepairKind,
    pub description: String,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairKind {
    InsertText,
    ReplaceText,
    DeleteText,
    AddImport,
    AddCapability,
    AddSanitizer,
}

/// Summary of effect analysis for the compilation unit.
#[derive(Debug, Serialize)]
pub struct EffectsSummary {
    pub functions_with_effects: usize,
    pub total_effects: usize,
    pub effects: Vec<FunctionEffects>,
}

/// Per-function effect summary.
#[derive(Debug, Serialize)]
pub struct FunctionEffects {
    pub function: String,
    pub effects: Vec<String>,
}

/// Summary of taint analysis.
#[derive(Debug, Serialize)]
pub struct TaintSummary {
    pub violations: usize,
    pub sources_tracked: usize,
}

impl JsonCompilationOutput {
    pub fn new_success() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            windjammer_version: env!("CARGO_PKG_VERSION").to_string(),
            success: true,
            diagnostics: Vec::new(),
            effects_summary: None,
            taint_summary: None,
        }
    }

    pub fn new_failure() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            windjammer_version: env!("CARGO_PKG_VERSION").to_string(),
            success: false,
            diagnostics: Vec::new(),
            effects_summary: None,
            taint_summary: None,
        }
    }

    pub fn add_error(&mut self, message: String) {
        self.diagnostics.push(JsonDiagnostic {
            code: None,
            severity: Severity::Error,
            message,
            span: None,
            maturity: None,
            repair_hint: None,
        });
    }

    pub fn add_warning(&mut self, message: String) {
        self.diagnostics.push(JsonDiagnostic {
            code: None,
            severity: Severity::Warning,
            message,
            span: None,
            maturity: None,
            repair_hint: None,
        });
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| {
            format!("{{\"error\": \"serialization failed: {}\"}}", e)
        })
    }

    pub fn emit_to_stdout(&self) {
        println!("{}", self.to_json());
    }
}

/// Convert IR diagnostics to JSON format.
pub fn from_ir_module(module: &crate::ir::pipeline::IrModule) -> JsonCompilationOutput {
    use crate::ir::pipeline::DiagnosticSeverity;

    let has_errors = module
        .diagnostics
        .iter()
        .any(|d| matches!(d.severity, DiagnosticSeverity::Error));

    let mut output = if has_errors {
        JsonCompilationOutput::new_failure()
    } else {
        JsonCompilationOutput::new_success()
    };

    for diag in &module.diagnostics {
        output.diagnostics.push(JsonDiagnostic {
            code: None,
            severity: match diag.severity {
                DiagnosticSeverity::Error => Severity::Error,
                DiagnosticSeverity::Warning => Severity::Warning,
                DiagnosticSeverity::Info => Severity::Info,
            },
            message: diag.message.clone(),
            span: diag.span.map(|(start, end)| DiagnosticSpan {
                file: String::new(),
                start_line: start,
                start_col: 0,
                end_line: end,
                end_col: 0,
            }),
            maturity: None,
            repair_hint: None,
        });
    }

    if !module.effect_sets.is_empty() {
        let effects: Vec<FunctionEffects> = module
            .effect_sets
            .iter()
            .map(|(fn_name, effect_set)| FunctionEffects {
                function: fn_name.clone(),
                effects: effect_set.iter().map(|e| e.to_string()).collect(),
            })
            .collect();
        let total: usize = effects.iter().map(|e| e.effects.len()).sum();
        output.effects_summary = Some(EffectsSummary {
            functions_with_effects: effects.len(),
            total_effects: total,
            effects,
        });
    }

    if !module.taint_errors.is_empty() {
        output.taint_summary = Some(TaintSummary {
            violations: module.taint_errors.len(),
            sources_tracked: 0,
        });
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_output_serialization() {
        let mut output = JsonCompilationOutput::new_success();
        output.add_warning("test warning".to_string());
        let json = output.to_json();
        assert!(json.contains("schema_version"));
        assert!(json.contains("test warning"));
        assert!(json.contains("\"success\": true"));
    }

    #[test]
    fn test_json_output_failure() {
        let mut output = JsonCompilationOutput::new_failure();
        output.add_error("parse error".to_string());
        let json = output.to_json();
        assert!(json.contains("\"success\": false"));
        assert!(json.contains("parse error"));
    }

    #[test]
    fn test_repair_hint_serialization() {
        let diag = JsonDiagnostic {
            code: Some("W0010".to_string()),
            severity: Severity::Warning,
            message: "non-canonical string type".to_string(),
            span: Some(DiagnosticSpan {
                file: "main.wj".to_string(),
                start_line: 5,
                start_col: 10,
                end_line: 5,
                end_col: 13,
            }),
            maturity: Some("Proven".to_string()),
            repair_hint: Some(RepairHint {
                kind: RepairKind::ReplaceText,
                description: "Replace `str` with `string`".to_string(),
                confidence: 0.95,
                replacement: Some("string".to_string()),
            }),
        };
        let json = serde_json::to_string(&diag).unwrap();
        assert!(json.contains("replace_text"));
        assert!(json.contains("0.95"));
    }
}
