//! Parser module for Windjammer
//!
//! This module is responsible for parsing Windjammer source code into an Abstract Syntax Tree (AST).
//! The parser is split into logical sub-modules for maintainability:
//!
//! - **helpers**: Utility functions for token manipulation
//! - **types**: Type parsing
//! - **expressions**: Expression parsing
//! - **statements**: Statement parsing
//! - **patterns**: Pattern matching parsing
//! - **items**: Top-level item parsing (functions, structs, enums, etc.)
//! - **traits**: Trait and impl block parsing
//! - **decorators**: Decorator parsing
//!
//! The main `Parser` struct provides the public API for parsing.

// Re-export the main parser from parser_impl
// The parser is well-organized in parser_impl.rs with clear sections:
// - AST Types
// - Parser Core & Utilities
// - Top-level Parsing
// - Item Parsing (functions, structs, enums, traits, impls)
// - Statement Parsing
// - Pattern Parsing
// - Expression Parsing
// - Type Parsing
//
// Future refactoring could split this into separate modules, but the current
// organization with section comments is clean and maintainable.
pub use crate::parser_impl::*;
