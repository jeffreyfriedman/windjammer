//! Top-level item generation for Rust code
//!
//! This module handles the generation of top-level items:
//! - Functions
//! - Structs
//! - Enums
//! - Traits
//! - Impl blocks
//! - Type aliases
//! - Constants
//! - etc.

use crate::parser::*;

/// Item generation methods for CodeGenerator
pub trait ItemGenerator {
    fn generate_function(&mut self, func: &FunctionDecl) -> String;
    fn generate_struct(&mut self, struct_decl: &StructDecl) -> String;
    fn generate_enum(&mut self, enum_decl: &EnumDecl) -> String;
    fn generate_trait(&mut self, trait_decl: &TraitDecl) -> String;
    fn generate_impl(&mut self, impl_block: &ImplBlock) -> String;
}

// Implementation will be added after extracting from generator.rs
