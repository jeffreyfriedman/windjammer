//! Variable Analysis Module
//!
//! Handles variable usage tracking, data flow analysis, borrow inference,
//! and mutability detection for code generation.
//!
//! Includes:
//! - For-loop borrow precomputation
//! - Unused binding detection
//! - Variable mutation analysis
//! - Iteration borrow semantics
//! - Self-reference detection for closures

pub(crate) use crate::codegen::rust::generator::CodeGenerator;

mod collection_element_inference;
mod for_loop_borrow_and_usize;
mod iteration_borrow_dispatch;
mod mut_inference;
mod usage_move_classification;
mod variable_usage_queries;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::codegen::rust) enum VariableUsage {
    NotUsed,
    FieldAccessOnly,
    Moved,
}
