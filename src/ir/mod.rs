//! Safety-Typed Intermediate Representation
//!
//! This module defines the IR layer that sits between the analyzer and codegen.
//! Every IR node carries a `SafetyType` encoding ownership, effects, taint,
//! execution mode, and const-eval status — solved once by the constraint system
//! and consumed by all backends.
//!
//! # Migration Strategy
//!
//! Phase 1 (current): Types are defined and `From<AnalyzedFunction>` bridges
//! the existing analyzer output into IR form. Codegen continues to read from
//! `AnalyzedFunction` directly; the IR types are available for new code.
//!
//! Phase 2+: The unified constraint solver populates IR nodes directly,
//! and codegen migrates to reading from IR instead of `AnalyzedFunction`.

pub mod annotations;
pub mod capability_lock;
pub mod capability_profiles;
pub mod cost_model;
pub mod constraint_gen;
pub mod constraints;
pub mod effects;
pub mod execution;
pub mod node;
pub mod numeric_bridge;
pub mod numeric_solver;
pub mod numeric_types;
pub mod optimizations;
pub mod pipeline;
pub mod safety_type;
pub mod shadow;
pub mod solver;
pub mod taint;
pub mod target_encodings;

pub use annotations::{CloneAnnotation, OptimizationHints};
pub use capability_profiles::{
    all_profiles, build_tool, database, file_processor, http_client, parser, profile_by_name,
    CapabilityProfile, TrustLevel,
};
pub use cost_model::{CompilationCostTracker, CompilationPhase};
pub use constraint_gen::{generate_constraints, generate_module_constraints, FunctionConstraints};
pub use constraints::{Constraint, ConstraintSet, ConstraintVar};
pub use effects::{EffectConstraint, EffectSolver, EffectSolverResult};
pub use execution::{ExecutionConstraint, ExecutionValidator};
pub use node::{parser_type_to_base_type, IrFunction, IrNode, IrNodeKind};
pub use numeric_solver::NumericSolver;
pub use numeric_types::{NumericConstraint, NumericType, UnifiedExprId};
pub use optimizations::{
    AoSoAPass, CostDelta, CowPass, DeferDropPass, IrOptimizationPass, OptimizationPipeline,
    OptimizationResult, PipelineResult, SmallVecPass,
};
pub use pipeline::{FileIrModule, IrModule, IrModuleSet, IrPipeline, IrPipelineConfig};
pub use safety_type::{
    BaseType, ConstEval, EffectSet, ExecutionMode, OwnedType, Region, SafetyType, TaintSource,
    TaintSourceKind, TaintStatus,
};
pub use solver::{CloneRequirements, Solver, SolverResult};
pub use taint::{TaintConstraint, TaintSolver, TaintSolverResult, TaintVar};
