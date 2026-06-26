//! Optimization annotations extracted from the analyzer.
//!
//! These map 1:1 to the optimization fields on `AnalyzedFunction`, providing
//! a typed bridge for codegen to query optimization hints from the IR.

use crate::analyzer::{
    AnalyzedFunction, CloneOptimization, ConstStaticOptimization, CowOptimization,
    DeferDropOptimization, SmallVecOptimization, StringOptimization, StructMappingOptimization,
};
use crate::auto_clone::AutoCloneAnalysis;

/// All optimization hints for a function, derived from analyzer phases 2-9.
#[derive(Debug, Clone)]
pub struct OptimizationHints {
    /// Phase 2: Clone elimination opportunities.
    pub clone_eliminations: Vec<CloneOptimization>,
    /// Phase 3: Struct mapping optimizations (FromRow, Builder, etc.).
    pub struct_mappings: Vec<StructMappingOptimization>,
    /// Phase 4: String capacity/concatenation optimizations.
    pub string_opts: Vec<StringOptimization>,
    /// Phase 6: Deferred drop for large allocations.
    pub defer_drops: Vec<DeferDropOptimization>,
    /// Phase 7: Promote runtime statics to compile-time consts.
    pub const_statics: Vec<ConstStaticOptimization>,
    /// Phase 8: SmallVec for bounded collections.
    pub smallvec: Vec<SmallVecOptimization>,
    /// Phase 9: Cow for conditionally-modified data.
    pub cow: Vec<CowOptimization>,
    /// Auto-clone insertion points.
    pub auto_clone: AutoCloneAnalysis,
}

impl OptimizationHints {
    pub fn from_analyzed(af: &AnalyzedFunction<'_>) -> Self {
        Self {
            clone_eliminations: af.clone_optimizations.clone(),
            struct_mappings: af.struct_mapping_optimizations.clone(),
            string_opts: af.string_optimizations.clone(),
            defer_drops: af.defer_drop_optimizations.clone(),
            const_statics: af.const_static_optimizations.clone(),
            smallvec: af.smallvec_optimizations.clone(),
            cow: af.cow_optimizations.clone(),
            auto_clone: af.auto_clone_analysis.clone(),
        }
    }

    pub fn empty() -> Self {
        Self {
            clone_eliminations: Vec::new(),
            struct_mappings: Vec::new(),
            string_opts: Vec::new(),
            defer_drops: Vec::new(),
            const_statics: Vec::new(),
            smallvec: Vec::new(),
            cow: Vec::new(),
            auto_clone: AutoCloneAnalysis::default(),
        }
    }
}

/// Annotation placed on IR nodes that need a clone inserted.
/// Replaces the heuristic auto-clone pass with a solver-driven decision.
#[derive(Debug, Clone, PartialEq)]
pub enum CloneAnnotation {
    /// No clone needed — value is used once or type is Copy.
    None,
    /// Clone required — value used at multiple sites with ownership transfer.
    NeedsClone,
    /// Clone can be eliminated — proven unnecessary by the solver.
    Eliminated(CloneEliminationReason),
}

/// Why a clone was eliminated (matches existing `CloneEliminationReason`).
#[derive(Debug, Clone, PartialEq)]
pub enum CloneEliminationReason {
    OnlyRead,
    SingleUse,
    LocalOnly,
    CanMove,
}
