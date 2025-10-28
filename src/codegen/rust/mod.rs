//! Rust code generation modules
//!
//! This module contains the Rust code generator split into logical components:
//! - **generator**: Main CodeGenerator coordinator
//! - **expressions**: Expression generation
//! - **statements**: Statement generation  
//! - **items**: Top-level item generation (functions, structs, enums, etc.)
//! - **types**: Type conversion (Windjammer → Rust)
//! - **optimizations**: Optimization passes
//! - **helpers**: Utility functions
//! - **backend**: Backend trait implementation

pub mod backend;
pub mod expressions;
pub mod generator;
pub mod helpers;
pub mod items;
pub mod optimizations;
pub mod statements;
pub mod types;

// Re-export the main CodeGenerator for backward compatibility
pub use generator::CodeGenerator;

// Re-export the backend
pub use backend::RustBackend;

// Re-export commonly used functions
pub use types::type_to_rust;
