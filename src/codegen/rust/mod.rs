//! Rust code generation modules
//!
//! This module contains the Rust code generator split into logical components:
//! - **generator**: Main CodeGenerator (formerly codegen_legacy.rs)
//! - **types**: Type conversion (Windjammer → Rust)
//! - **backend**: Backend trait implementation
//!
//! Future refactoring will extract:
//! - expressions: Expression generation
//! - statements: Statement generation
//! - patterns: Pattern matching
//! - functions: Function generation

pub mod backend;
pub mod generator;
pub mod types;

// Re-export the main CodeGenerator for backward compatibility
pub use generator::CodeGenerator;

// Re-export the backend
pub use backend::RustBackend;

// Re-export commonly used functions
pub use types::type_to_rust;
