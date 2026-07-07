//! IR-to-IR optimization passes for WJ-PERF-01 economic efficiency.
//!
//! Each pass transforms [`IrFunction`] metadata (and eventually IR nodes)
//! while reporting estimated cost savings via [`CostDelta`].

use crate::analyzer::{CowOptimization, DeferDropOptimization, EstimatedSize, SmallVecOptimization};
use crate::ir::node::IrFunction;

/// Estimated change in compilation and runtime economics from a pass.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CostDelta {
    pub compile_time_ms: f64,
    pub estimated_runtime_ns: f64,
    pub memory_bytes: i64,
    pub binary_size_bytes: i64,
}

impl CostDelta {
    pub const fn zero() -> Self {
        Self {
            compile_time_ms: 0.0,
            estimated_runtime_ns: 0.0,
            memory_bytes: 0,
            binary_size_bytes: 0,
        }
    }

    pub fn accumulate(&mut self, other: &CostDelta) {
        self.compile_time_ms += other.compile_time_ms;
        self.estimated_runtime_ns += other.estimated_runtime_ns;
        self.memory_bytes += other.memory_bytes;
        self.binary_size_bytes += other.binary_size_bytes;
    }
}

/// Result of applying a single IR optimization pass to one function.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OptimizationResult {
    pub transformations: usize,
    pub estimated_savings: CostDelta,
}

impl OptimizationResult {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with_transformations(count: usize, savings: CostDelta) -> Self {
        Self {
            transformations: count,
            estimated_savings: savings,
        }
    }
}

/// IR-to-IR optimization pass.
pub trait IrOptimizationPass {
    fn name(&self) -> &'static str;
    fn apply(&self, ir: &mut IrFunction) -> OptimizationResult;
}

/// Delay drop calls to end of scope for better codegen.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeferDropPass;

impl IrOptimizationPass for DeferDropPass {
    fn name(&self) -> &'static str {
        "defer_drop"
    }

    fn apply(&self, ir: &mut IrFunction) -> OptimizationResult {
        let hints = &ir.optimizations.defer_drops;
        if hints.is_empty() {
            return OptimizationResult::none();
        }

        let savings = estimate_defer_drop_savings(hints);
        // Stub: actual IR rewrite would reorder drop sites; hints already populated
        // by the analyzer and consumed by codegen.
        OptimizationResult::with_transformations(hints.len(), savings)
    }
}

/// Replace `Vec<T>` with `SmallVec<[T; N]>` where N items fit inline.
#[derive(Debug, Clone, Copy, Default)]
pub struct SmallVecPass;

impl IrOptimizationPass for SmallVecPass {
    fn name(&self) -> &'static str {
        "smallvec"
    }

    fn apply(&self, ir: &mut IrFunction) -> OptimizationResult {
        let hints = &ir.optimizations.smallvec;
        if hints.is_empty() {
            return OptimizationResult::none();
        }

        let savings = estimate_smallvec_savings(hints);
        OptimizationResult::with_transformations(hints.len(), savings)
    }
}

/// Replace owned strings with `Cow<'_, str>` where the string is never mutated.
#[derive(Debug, Clone, Copy, Default)]
pub struct CowPass;

impl IrOptimizationPass for CowPass {
    fn name(&self) -> &'static str {
        "cow"
    }

    fn apply(&self, ir: &mut IrFunction) -> OptimizationResult {
        let hints: Vec<&CowOptimization> = ir
            .optimizations
            .cow
            .iter()
            .filter(|opt| !ir.mutated_locals.contains(&opt.variable))
            .collect();

        if hints.is_empty() {
            return OptimizationResult::none();
        }

        let savings = estimate_cow_savings(&hints);
        OptimizationResult::with_transformations(hints.len(), savings)
    }
}

/// Array-of-structs-of-arrays layout transformation (future placeholder).
#[derive(Debug, Clone, Copy, Default)]
pub struct AoSoAPass;

impl IrOptimizationPass for AoSoAPass {
    fn name(&self) -> &'static str {
        "aosoa"
    }

    fn apply(&self, ir: &mut IrFunction) -> OptimizationResult {
        let candidates = &ir.optimizations.cache_locality.aosoa_candidates;
        if candidates.is_empty() {
            return OptimizationResult::none();
        }

        // Stub: future pass will rewrite loop iteration to SoA/AoSoA layouts.
        let savings = CostDelta {
            compile_time_ms: 0.05 * candidates.len() as f64,
            estimated_runtime_ns: 500.0 * candidates.len() as f64,
            memory_bytes: 0,
            binary_size_bytes: 128 * candidates.len() as i64,
        };
        OptimizationResult::with_transformations(candidates.len(), savings)
    }
}

/// Runs a ordered sequence of IR optimization passes over every function.
pub struct OptimizationPipeline {
    passes: Vec<Box<dyn IrOptimizationPass + Send + Sync>>,
}

impl Default for OptimizationPipeline {
    fn default() -> Self {
        Self::default_passes()
    }
}

impl OptimizationPipeline {
    pub fn default_passes() -> Self {
        Self {
            passes: vec![
                Box::new(DeferDropPass),
                Box::new(SmallVecPass),
                Box::new(CowPass),
                Box::new(AoSoAPass),
            ],
        }
    }

    pub fn with_passes(passes: Vec<Box<dyn IrOptimizationPass + Send + Sync>>) -> Self {
        Self { passes }
    }

    pub fn apply_to_functions(&self, functions: &mut [IrFunction]) -> PipelineResult {
        let mut result = PipelineResult::default();
        for ir in functions.iter_mut() {
            for pass in &self.passes {
                let pass_result = pass.apply(ir);
                result.record_pass(pass.name(), pass_result);
            }
        }
        result
    }
}

/// Aggregated results from running an [`OptimizationPipeline`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PipelineResult {
    pub total_transformations: usize,
    pub total_savings: CostDelta,
    pub per_pass: Vec<(String, OptimizationResult)>,
}

impl PipelineResult {
    fn record_pass(&mut self, name: &str, result: OptimizationResult) {
        self.total_transformations += result.transformations;
        self.total_savings.accumulate(&result.estimated_savings);
        self.per_pass.push((name.to_string(), result));
    }
}

fn estimate_defer_drop_savings(hints: &[DeferDropOptimization]) -> CostDelta {
    let mut savings = CostDelta::zero();
    for hint in hints {
        let (runtime_ns, memory_bytes) = match hint.estimated_size {
            EstimatedSize::Small => (0.0, 0),
            EstimatedSize::Medium => (1_000.0, -512),
            EstimatedSize::Large => (10_000.0, -4_096),
            EstimatedSize::VeryLarge => (100_000.0, -16_384),
        };
        savings.estimated_runtime_ns += runtime_ns;
        savings.memory_bytes += memory_bytes;
        savings.compile_time_ms += 0.01;
    }
    savings
}

fn estimate_smallvec_savings(hints: &[SmallVecOptimization]) -> CostDelta {
    let mut savings = CostDelta::zero();
    for hint in hints {
        let heap_bytes = (hint.estimated_max_size * std::mem::size_of::<usize>()) as i64;
        savings.memory_bytes -= heap_bytes.max(64);
        savings.estimated_runtime_ns += 50.0 * hint.estimated_max_size as f64;
        savings.binary_size_bytes += 32;
        savings.compile_time_ms += 0.005;
    }
    savings
}

fn estimate_cow_savings(hints: &[&CowOptimization]) -> CostDelta {
    let mut savings = CostDelta::zero();
    for _hint in hints {
        savings.memory_bytes -= 64;
        savings.estimated_runtime_ns += 200.0;
        savings.binary_size_bytes += 16;
        savings.compile_time_ms += 0.002;
    }
    savings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{
        AccessPatternKind, AoSoACandidate, CacheLocalityAnalysis, CowReason, DeferDropReason,
    };
    use crate::ir::annotations::OptimizationHints;
    use crate::ir::safety_type::{BaseType, SafetyType};
    use std::collections::{HashMap, HashSet};

    fn stub_ir_function(hints: OptimizationHints) -> IrFunction {
        IrFunction {
            name: "test_fn".to_string(),
            param_types: HashMap::new(),
            return_type: SafetyType::owned(BaseType::Unit),
            mutated_locals: HashSet::new(),
            mutated_params: HashSet::new(),
            str_ref_params: HashSet::new(),
            optimizations: hints,
        }
    }

    #[test]
    fn defer_drop_pass_counts_hints() {
        let mut ir = stub_ir_function(OptimizationHints {
            defer_drops: vec![DeferDropOptimization {
                variable: "big".to_string(),
                estimated_size: EstimatedSize::Large,
                reason: DeferDropReason::LargeLocalVariable,
                location: 0,
            }],
            ..OptimizationHints::empty()
        });

        let pass = DeferDropPass;
        let result = pass.apply(&mut ir);
        assert_eq!(result.transformations, 1);
        assert!(result.estimated_savings.estimated_runtime_ns > 0.0);
    }

    #[test]
    fn cow_pass_skips_mutated_locals() {
        let mut ir = stub_ir_function(OptimizationHints {
            cow: vec![CowOptimization {
                variable: "label".to_string(),
                reason: CowReason::ReadHeavy,
            }],
            ..OptimizationHints::empty()
        });
        ir.mutated_locals.insert("label".to_string());

        let pass = CowPass;
        let result = pass.apply(&mut ir);
        assert_eq!(result.transformations, 0);
    }

    #[test]
    fn pipeline_runs_all_default_passes() {
        let mut functions = vec![stub_ir_function(OptimizationHints {
            cache_locality: CacheLocalityAnalysis {
                aosoa_candidates: vec![AoSoACandidate {
                    function_name: "test_fn".to_string(),
                    loop_var: "e".to_string(),
                    iterable_var: "entities".to_string(),
                    element_struct: "Entity".to_string(),
                    field_access_counts: vec![("x".to_string(), 4)],
                    hot_fields: vec!["x".to_string()],
                    cold_fields: vec![],
                    pattern_kind: AccessPatternKind::SequentialIteration,
                    simd_friendly_layout: true,
                }],
            },
            ..OptimizationHints::empty()
        })];

        let pipeline = OptimizationPipeline::default_passes();
        let result = pipeline.apply_to_functions(&mut functions);
        assert_eq!(result.per_pass.len(), 4);
        assert!(result.total_transformations >= 1);
    }
}
