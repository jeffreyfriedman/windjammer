//! Statement generation for Rust code
//!
//! This module handles the generation of all statement types:
//! - Variable declarations (let, const, static)
//! - Assignments
//! - If/while/for/match statements
//! - Return statements
//! - Expression statements
//! - etc.

use crate::parser::*;

/// Statement generation methods for CodeGenerator
pub trait StatementGenerator {
    fn generate_statement(&mut self, stmt: &Statement) -> String;
}

// Implementation will be added after extracting from generator.rs
