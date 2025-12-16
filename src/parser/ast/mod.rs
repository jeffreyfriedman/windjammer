// AST Module - Windjammer Abstract Syntax Tree
//
// This module is being refactored into domain-specific submodules.
// All types are re-exported to maintain backward compatibility.

// Import legacy types from ast_legacy.rs (temporary during migration)
#[path = "../ast_legacy.rs"]
mod ast_legacy;

// Domain modules (extracted)
pub mod types;
pub mod literals;
pub mod operators;

// Re-export from domain modules
pub use types::*;
pub use literals::*;
pub use operators::*;

// Re-export non-type system items from legacy ast.rs
// TODO: Extract these into their own modules
pub use ast_legacy::{
    Decorator, EnumDecl, EnumPatternBinding, EnumVariant, EnumVariantData, Expression,
    FunctionDecl, ImplBlock, Item, MatchArm, OwnershipHint, Parameter, Pattern, Program,
    Statement, StructDecl, StructField, TraitDecl, TraitMethod,
};
