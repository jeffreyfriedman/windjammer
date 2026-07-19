//! Shared IR analysis context for all codegen backends.
//!
//! Runs Analyzer + IrPipeline once and attaches the resulting `IrModule`
//! for consumption by Rust, Go, JavaScript, and WASM backends.

use crate::analyzer::{Analyzer, SignatureRegistry};
use crate::ir::pipeline::{IrModule, IrPipeline};
use crate::parser::Program;
use anyhow::Result;

/// Analysis + IR lowering results shared across codegen backends.
#[derive(Debug)]
pub struct IrContext<'ast> {
    pub analyzed: Vec<crate::analyzer::AnalyzedFunction<'ast>>,
    pub registry: SignatureRegistry,
    pub module: IrModule,
}

/// Run analyzer and IR pipeline for a program.
pub fn analyze_and_lower<'ast>(program: &Program<'ast>) -> Result<IrContext<'ast>> {
    let mut analyzer = Analyzer::new();
    analyzer
        .check_forbidden_rust_patterns(program)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let (analyzed, registry, _) = analyzer
        .analyze_program(program)
        .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;

    let mut pipeline = IrPipeline::new();
    let module = pipeline.lower_to_ir(&analyzed, &registry, None);

    Ok(IrContext {
        analyzed,
        registry,
        module,
    })
}
