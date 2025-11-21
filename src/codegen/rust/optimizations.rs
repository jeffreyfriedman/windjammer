//! Optimization passes for Rust code generation
//!
//! This module contains various optimization passes:
//! - Constant folding
//! - Dead code elimination
//! - String capacity hints
//! - SmallVec optimizations
//! - Cow optimizations
//! - Clone elimination
//! - etc.

use crate::parser::*;

/// Optimization methods for CodeGenerator
pub trait OptimizationGenerator {
    fn try_fold_constant(&self, expr: &Expression) -> Option<Expression>;
}

// Implementation will be added after extracting from generator.rs
