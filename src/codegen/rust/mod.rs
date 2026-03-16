//! Rust code generation modules
//!
//! This module contains the Rust code generator split into logical components:
//! - **generator**: Main CodeGenerator coordinator
//! - **expressions**: Expression generation
//! - **statements**: Statement generation  
//! - **items**: Top-level item generation (functions, structs, enums, etc.)
//! - **types**: Type conversion (Windjammer → Rust)
//! - **literals**: Literal expression generation (pure functions)
//! - **optimizations**: Optimization passes
//! - **helpers**: Utility functions
//! - **backend**: Backend trait implementation

pub mod arm_string_analysis;
pub mod ast_utilities;
pub mod backend;
pub mod codegen_helpers;
pub mod collection_detection;
pub mod constant_folding;
pub mod copy_semantics;
pub mod expression_generation;
pub mod expression_helpers;
pub mod expressions;
pub mod function_generation;
pub mod generator;
pub mod helpers;
pub mod import_generation;
pub mod item_generation;
pub mod items;
pub mod literals;
pub mod operators;
pub mod optimizations;
pub mod ownership_tracker;
pub mod pattern_analysis;
pub mod rust_coercion_rules;
pub mod self_analysis;
pub mod statement_generation;
pub mod statements;
pub mod string_analysis;
pub mod trait_derivation;
pub mod type_analysis;
pub mod type_casting;
pub mod types;
pub mod variable_analysis;

// Re-export the main CodeGenerator for backward compatibility
pub use generator::CodeGenerator;

// Re-export the backend
pub use backend::RustBackend;

// Re-export commonly used functions
pub use types::type_to_rust;
pub mod method_call_analyzer;
