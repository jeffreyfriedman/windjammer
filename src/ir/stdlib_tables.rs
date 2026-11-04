//! The stdlib's effect declarations and taint sources/sinks, and the typed surface of the Rust core types. They are
//! functions of the stdlib only, yet each `IrPipeline` used to rebuild them (once per checked file). A compilation
//! session computes them once (`StdlibTables::compute`) and hands the same tables to all its pipelines
//! (`IrPipeline::with_stdlib_tables`).

use super::effects::{stdlib_effect_declarations, EffectConstraint};
use super::taint::{stdlib_sinks, stdlib_taint_sources, TaintConstraint};
use crate::typing::core_surface::CoreSurface;

#[derive(Debug, Clone)]
pub struct StdlibTables {
    pub effect_declarations: Vec<EffectConstraint>,
    pub taint_sources: Vec<TaintConstraint>,
    pub taint_sinks: Vec<TaintConstraint>,
    /// `std/rust_core.wji` (option C stage C1): core-type method signatures for the numeric solver.
    pub core_surface: CoreSurface,
}

impl StdlibTables {
    pub fn compute() -> Self {
        Self {
            effect_declarations: stdlib_effect_declarations(),
            taint_sources: stdlib_taint_sources(),
            taint_sinks: stdlib_sinks(),
            core_surface: CoreSurface::parse(),
        }
    }
}
