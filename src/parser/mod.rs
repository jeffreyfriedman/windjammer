//! Windjammer Parser
//!
//! This module contains the parser for the Windjammer programming language.
//! The parser is organized into several submodules for better maintainability:
//!
//! - `ast`: Abstract Syntax Tree type definitions
//! - `core`: Core parser struct and methods
//!
//! Expression parsing is split across `binary_expression_parser`, `call_expression_parser`,
//! `compound_primary_expression_parser`, `primary_expression_parser`, plus
//! `interpolated_string_expression_parser` and `match_value_expression_parser`.

// AST module - extracted from parser_impl.rs
pub mod ast;

// Type parsing module - extracted from parser_impl.rs
pub mod type_parser;

// Pattern parsing module - extracted from parser_impl.rs
pub mod pattern_parser;

// Expression parsing (split across submodules; all extend `impl Parser`)
mod binary_expression_parser;
mod call_expression_parser;
mod compound_primary_expression_parser;
pub mod expression_parser;
mod interpolated_string_expression_parser;
mod match_value_expression_parser;
mod primary_expression_parser;

// Statement parsing module - extracted from parser_impl.rs
pub mod statement_parser;

// Item sub-parsers (split from item_parser for maintainability)
pub mod enum_parser;
pub mod function_parser;
pub mod struct_parser;
pub mod trait_parser;

// Item parsing module - extracted from parser_impl.rs
pub mod item_parser;

// Re-export AST types for convenience
pub use ast::*;

// Re-export everything else from parser_impl for now to maintain backward compatibility
pub use crate::parser_impl::ParseWarning;
pub use crate::parser_impl::Parser;

// TODO: Uncomment these as we create the modules
// pub mod core;
// pub mod helpers;
