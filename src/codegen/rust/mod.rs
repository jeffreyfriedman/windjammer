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

pub mod aosoa_transform;
pub mod arm_string_analysis;
pub mod assignment_statement_generation;
pub mod ast_utilities;
pub mod async_generation;
pub mod auto_super_type_imports;
pub mod backend;
pub mod binary_expression_generation;
pub mod block_generation;
pub mod call_expression_generation;
pub mod call_signature_resolution;
pub mod closure_generation;
pub mod codegen_helpers;
pub mod collection_detection;
pub mod const_static_generation;
pub mod constant_folding;
pub mod copy_semantics;
pub mod data_structure_generation;
pub mod defer_drop_generation;
pub mod expression_generation;
pub mod expression_helpers;
mod expression_type_inference;
pub mod expression_utilities;
pub mod expressions;
pub mod float_type_utilities;
pub mod for_statement_generation;
mod function_extern_generation;
mod function_formal_parameter_generation;
pub mod function_generation;
mod function_generation_body;
mod function_generation_prepare;
mod function_generation_signature;
mod function_implicit_self_generation;
mod function_parameterized_tests_generation;
mod function_self_helpers;
mod function_wrapping_generation;
pub mod generator;
pub mod generator_type_formatting;
pub mod helpers;
pub mod identifier_and_literal_generation;
pub mod if_statement_generation;
pub mod import_generation;
mod int_promotion_type_inference;
pub mod item_generation;
pub mod items;
pub mod let_statement_generation;
pub mod literals;
pub mod loop_statement_generation;
pub mod macro_and_string_generation;
pub mod macro_conversion;
mod match_binding_type_inference;
pub mod match_statement_generation;
pub mod method_signature;
pub mod operator_generation;
pub mod operators;
pub mod optimizations;
pub mod ownership_tracker;
pub mod pattern_analysis;
pub mod pattern_generation;
pub mod program_generation;
pub mod return_statement_generation;
pub mod simd_transform;
pub mod rust_coercion_rules;
pub mod self_analysis;
pub mod statement_generation;
pub mod statement_mut_binding_scan;
pub mod statements;
pub mod stdlib_method_signatures;
pub mod string_analysis;
pub mod string_utilities;
pub mod thread_async_generation;
pub mod trait_derivation;
pub mod type_analysis;
pub mod type_analysis_pure;
pub mod type_analyzer;
pub mod type_balancing;
pub mod type_casting;
pub mod type_classification_utilities;
mod type_name_inference;
pub mod types;
mod usize_expression_type_inference;
pub mod variable_analysis;

// Re-export the main CodeGenerator for backward compatibility
pub use generator::CodeGenerator;

// Re-export the backend
pub use backend::RustBackend;

// Re-export commonly used functions
pub use types::type_to_rust;
pub mod method_call_analyzer;
pub mod method_call_expression_generation;
pub mod rust_stdlib_annotations;
pub mod stdlib_method_traits;
