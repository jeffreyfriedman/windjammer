// Allow recursive functions that use self only for recursion
// This is common in AST traversal code
#![allow(clippy::only_used_in_recursion)]

use clap::ValueEnum;

pub mod analyzer;
pub mod auto_clone;
pub mod codegen;
pub mod component_analyzer;
pub mod error;
pub mod errors;
pub mod inference;
pub mod interpreter;
pub mod lexer;
pub mod linter;
pub mod metadata;
pub mod parser;
pub mod parser_impl;
pub mod source_map;
pub mod stdlib_scanner;
pub mod test_utils;
pub mod type_inference;
pub mod type_registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CompilationTarget {
    /// Native Rust binary
    Rust,
    /// Go source code (experimental)
    Go,
    /// WebAssembly
    Wasm,
    /// Node.js native modules (future)
    Node,
    /// Python FFI via PyO3 (future)
    Python,
    /// C FFI (future)
    C,
}
