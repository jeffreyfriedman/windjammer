// AST Module - Windjammer Abstract Syntax Tree
//
// Organized into domain-specific submodules:
// - Independent types (types, literals, operators, ownership)
// - Circular core (core.rs: Expression ↔ Statement ↔ Pattern)

// Core module with circular dependencies
pub mod core;

// Domain modules (extracted - independent, no circular deps)
pub mod types;
pub mod literals;
pub mod operators;
pub mod ownership;

// Re-export from domain modules
pub use types::*;
pub use literals::*;
pub use operators::*;
pub use ownership::*;

// Re-export circular types from core module
// These types have circular dependencies and must stay together:
// Expression ↔ Statement ↔ Pattern
pub use core::{
    Decorator, EnumDecl, EnumPatternBinding, EnumVariant, EnumVariantData, Expression,
    FunctionDecl, ImplBlock, Item, MatchArm, Parameter, Pattern, Program, Statement, StructDecl,
    StructField, TraitDecl, TraitMethod,
};
