//! Windjammer Parser
//!
//! This module contains the parser for the Windjammer programming language.
//! The parser is organized into several submodules for better maintainability:
//!
//! - `ast`: Abstract Syntax Tree type definitions
//! - `core`: Core parser struct and methods
//!
//! This is a work in progress - modules will be added incrementally as we
//! refactor the monolithic parser_impl.rs file.

// AST module - extracted from parser_impl.rs
pub mod ast;

// Type parsing module - extracted from parser_impl.rs
pub mod type_parser;

// Pattern parsing module - extracted from parser_impl.rs
pub mod pattern_parser;

// Expression parsing module - extracted from parser_impl.rs
pub mod expression_parser;

// Statement parsing module - extracted from parser_impl.rs
pub mod statement_parser;

// Re-export AST types for convenience
pub use ast::*;

// Re-export everything else from parser_impl for now to maintain backward compatibility
pub use crate::parser_impl::Parser;

// TODO: Uncomment these as we create the modules
// pub mod core;
// pub mod items;
// pub mod helpers;
