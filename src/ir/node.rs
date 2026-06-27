//! IR node definitions.
//!
//! IR nodes wrap AST declarations with safety type information.
//! During Phase 1, `IrFunction` is constructed from `AnalyzedFunction`
//! as a lossless bridge. In later phases, the constraint solver will
//! populate IR nodes directly.

use crate::analyzer::{AnalyzedFunction, OwnershipMode};
use crate::ir::annotations::OptimizationHints;
use crate::ir::safety_type::{BaseType, OwnedType, Region, SafetyType};
use std::collections::{HashMap, HashSet};

/// An IR-level function — carries safety types for all parameters and locals.
#[derive(Debug, Clone)]
pub struct IrFunction {
    /// Function name (qualified if in an impl block).
    pub name: String,

    /// Safety type of each parameter, keyed by parameter name.
    pub param_types: HashMap<String, SafetyType>,

    /// Safety type of the return value.
    pub return_type: SafetyType,

    /// Which local variables are mutated (derived from analyzer).
    pub mutated_locals: HashSet<String>,

    /// Which parameters are mutated (derived from analyzer).
    pub mutated_params: HashSet<String>,

    /// Parameters eligible for &str optimization.
    pub str_ref_params: HashSet<String>,

    /// Optimization annotations (derived from analyzer phases 2-9).
    pub optimizations: OptimizationHints,
}

impl IrFunction {
    /// Bridge from the existing `AnalyzedFunction` — lossless conversion.
    /// This is the Phase 1 facade: all data comes from the analyzer unchanged.
    pub fn from_analyzed(analyzed: &AnalyzedFunction<'_>) -> Self {
        let name = analyzed.decl.name.to_string();

        let param_types = analyzed
            .inferred_ownership
            .iter()
            .map(|(param_name, mode)| {
                let ownership = match mode {
                    OwnershipMode::Owned => OwnedType::Owned,
                    OwnershipMode::Borrowed => OwnedType::Ref(Region::fresh(0)),
                    OwnershipMode::MutBorrowed => OwnedType::MutRef(Region::fresh(0)),
                };
                let safety = SafetyType {
                    base: BaseType::Inferred,
                    ownership,
                    effects: crate::ir::safety_type::EffectSet::pure(),
                    taint: crate::ir::safety_type::TaintStatus::Clean,
                    const_eval: crate::ir::safety_type::ConstEval::Runtime,
                    exec_mode: None,
                };
                (param_name.clone(), safety)
            })
            .collect();

        let return_type = SafetyType::owned(BaseType::Inferred);

        let optimizations = OptimizationHints::from_analyzed(analyzed);

        IrFunction {
            name,
            param_types,
            return_type,
            mutated_locals: analyzed.mutated_variables.clone(),
            mutated_params: analyzed.mutated_parameters.clone(),
            str_ref_params: analyzed.str_ref_optimizable_params.clone(),
            optimizations,
        }
    }
}

/// A single IR node in the function body (future use).
/// During Phase 1 the body is not represented in the IR — codegen still reads
/// the AST directly. Phase 2+ will add expression-level IR nodes.
#[derive(Debug, Clone)]
pub struct IrNode {
    pub kind: IrNodeKind,
    pub safety_type: SafetyType,
}

/// IR node kinds (will expand significantly in Phase 2+).
#[derive(Debug, Clone)]
pub enum IrNodeKind {
    /// Placeholder for the current AST-direct codegen path.
    AstPassthrough,
    /// A call expression with resolved signature.
    Call { callee: String, args: Vec<IrNode> },
    /// A local variable binding.
    Let { name: String, mutable: bool },
    /// A field access.
    FieldAccess { base: Box<IrNode>, field: String },
}
