//! `wj explain-ownership`: report what ownership / clone / width inference decided for a
//! program and why, in terms of the rule IDs in `docs/spec/inference.md`.
//!
//! This module only *reports* (ADR-0001). It re-reads the same constraint set the IR pipeline
//! solves (`constraint_gen`), the IR result (`IrModule`), the analyzer's pre-decisions and,
//! when available, the emitted Rust, and shows where those stages disagree. Nothing computed
//! here flows back into compilation.

pub mod calls;
pub mod emitted;
pub mod params;
pub mod render;
pub mod report;
pub mod rules;

use crate::analyzer::Analyzer;
use crate::ir::constraint_gen::generate_constraints_with_fields;
use crate::ir::node::ir_function_name_from_decl;
use crate::ir::IrPipeline;
use report::{FunctionReport, Report};

/// Options for [`explain_source`].
#[derive(Debug, Clone, Default)]
pub struct ExplainOptions {
    /// Only report this function (bare name or `Type::name`).
    pub only_fn: Option<String>,
    /// Emitted Rust for the same source (from `wj build`), to fill the `emitted` columns.
    pub emitted_rust: Option<String>,
}

/// Analyze `source` and report the inference decisions. `file` is only a label.
pub fn explain_source(source: &str, file: &str, opts: &ExplainOptions) -> Result<Report, String> {
    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = crate::parser::Parser::new(tokens);
    let program = parser.parse().map_err(|e| format!("parse error: {e}"))?;

    let mut analyzer = Analyzer::new();
    let (analyzed, registry, _) = analyzer
        .analyze_program(&program)
        .map_err(|e| format!("analysis error: {e}"))?;
    let module = IrPipeline::new().lower_to_ir(&analyzed, &registry, None);

    let emitted_file = opts.emitted_rust.as_deref().and_then(emitted::parse_emitted);
    let mut notes = Vec::new();
    if opts.emitted_rust.is_some() && emitted_file.is_none() {
        notes.push("emitted Rust did not parse; `emitted` columns omitted".to_string());
    }

    let mut functions = Vec::new();
    for (i, af) in analyzed.iter().enumerate() {
        let name = ir_function_name_from_decl(&af.decl.name, af.decl.parent_type.as_deref());
        if let Some(only) = &opts.only_fn {
            if *only != name && *only != af.decl.name {
                continue;
            }
        }
        let Some(ir_fn) = module.functions.get(i) else { continue };
        let fc = generate_constraints_with_fields(af, Some(&registry), None);
        let em = emitted_file.as_ref().and_then(|f| f.fns.get(&name));
        let calls = match (em, emitted_file.as_ref()) {
            (Some(e), Some(f)) => calls::call_reports(e, f),
            _ => Vec::new(),
        };
        functions.push(FunctionReport {
            name,
            params: params::param_reports(af, &fc, ir_fn, em),
            locals: params::local_reports(&fc, ir_fn, em)
                .into_iter()
                .filter(|l| !l.name.contains("::") && (em.is_none() || l.mutable_in_emitted.is_some()))
                .collect(),
            clones: params::clone_reports(&fc, af, em),
            calls,
        });
    }
    if let Some(only) = &opts.only_fn {
        if functions.is_empty() {
            return Err(format!("no function named `{only}` in {file}"));
        }
    }
    Ok(Report {
        file: file.to_string(),
        has_emitted: emitted_file.is_some(),
        functions,
        notes,
    })
}
