//! Structured JSON diagnostics. The schema lives in `crate::diagnostics`
//! (docs/diagnostics-schema.json); this module only adapts IR solver output to it.

pub use crate::diagnostics::{Report, SCHEMA_VERSION};

use crate::diagnostics::convert;
use crate::ir::pipeline::{DiagnosticSeverity, IrModule};

/// IR diagnostics (effects, taint, execution) plus the effects / taint summaries.
pub fn from_ir_module(module: &IrModule) -> Report {
    let diags = module
        .diagnostics
        .iter()
        .map(|d| {
            convert::ir(
                d.severity == DiagnosticSeverity::Error,
                d.severity == DiagnosticSeverity::Warning,
                &d.message,
            )
        })
        .collect();
    let mut report = Report::new("build", diags);
    if !module.effect_sets.is_empty() {
        let effects: Vec<_> = module
            .effect_sets
            .iter()
            .map(|(f, set)| {
                serde_json::json!({
                    "function": f,
                    "effects": set.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                })
            })
            .collect();
        report.effects_summary = Some(serde_json::json!({
            "functions_with_effects": effects.len(),
            "effects": effects,
        }));
    }
    if !module.taint_errors.is_empty() {
        report.taint_summary = Some(serde_json::json!({ "violations": module.taint_errors.len() }));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Diagnostic;

    #[test]
    fn report_serializes_with_schema_version_and_success_flag() {
        let ok = Report::new("build", vec![Diagnostic::warning("W0007", "w")]);
        assert!(ok.success);
        let bad = Report::new("build", vec![Diagnostic::error("WJ0001", "e")]);
        let v: serde_json::Value = serde_json::from_str(&bad.to_json()).unwrap();
        assert_eq!(v["schema_version"], 2);
        assert_eq!(v["success"], false);
        assert_eq!(v["summary"]["errors"], 1);
    }
}
