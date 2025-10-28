//! Expression generation for Rust code
//!
//! This module handles the generation of all expression types:
//! - Binary and unary operations
//! - Function calls
//! - String concatenation
//! - Literals
//! - Identifiers
//! - Closures
//! - etc.

use crate::parser::*;

/// Expression generation methods for CodeGenerator
pub trait ExpressionGenerator {
    fn generate_expression(&mut self, expr: &Expression) -> String;
    fn generate_string_concat(&mut self, left: &Expression, right: &Expression) -> String;
}

// Implementation will be added after extracting from generator.rs
