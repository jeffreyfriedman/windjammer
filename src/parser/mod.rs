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

// Re-export everything from parser_impl for now to maintain backward compatibility
pub use crate::parser_impl::*;

// TODO: Uncomment these as we create the modules
// pub mod ast;
// pub mod core;
// pub mod types;
// pub mod patterns;
// pub mod expressions;
// pub mod statements;
// pub mod items;
// pub mod helpers;
