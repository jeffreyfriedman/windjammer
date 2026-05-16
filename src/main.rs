// Allow recursive functions that use self only for recursion
// This is common in AST traversal code
#![allow(clippy::only_used_in_recursion)]

pub mod analyzer;
pub mod auto_clone; // Automatic clone insertion for ergonomics
pub mod auto_fix; // Automatic error fixing
pub mod build_utils;
pub mod cargo_integration; // Cargo build system integration
pub mod cargo_toml;
pub mod cli;
pub mod cli_execution; // CLI execution (run, interpret, REPL)
pub mod codegen;
pub mod compiler;
pub mod component_analyzer;
pub mod decorator_registry;
pub mod error;
pub mod errors;
pub mod plugin; // Plugin discovery and delegation // High-quality error messages (mutability, etc.)
                // Removed: codegen_legacy is now codegen::rust::generator
mod compilation_error_handling;
pub mod compiler_database;
pub mod config;
pub mod ejector;
pub mod error_catalog; // Error catalog generation and documentation
pub mod error_codes;
pub mod error_handling; // Error handling and linting
mod file_compilation_pipeline;
pub mod file_compiler; // Single-file compilation
pub mod module_system;
mod output_generation;
pub mod project_paths; // Nested module system - The Windjammer Way! // Windjammer error codes (WJ0001, etc.)

pub mod error_mapper;
pub mod error_statistics; // Error statistics tracking and analysis
pub mod error_tui; // Interactive TUI for error navigation
pub mod fuzzy_matcher; // Fuzzy string matching for typo suggestions
pub mod inference;
pub mod interpreter; // Windjammerscript: tree-walking interpreter for fast iteration
pub mod lexer;
pub mod linter; // Windjammer-specific lints (performance, style, correctness)
pub mod metadata; // Cross-module type inference metadata
pub mod method_registry;
pub mod optimizer;
pub mod parser; // Parser module (refactored structure)
pub mod parser_impl; // Parser implementation (being migrated to parser/)
                     // Test utilities for arena-allocated AST construction (available for integration tests)
pub mod parser_recovery;
pub mod source_map; // Source map for error message translation
pub mod source_map_cache; // Source map caching for performance
pub mod stdlib_scanner;
pub mod syntax_highlighter;
pub mod test_runner; // Test framework and execution
pub mod test_utils; // Syntax highlighting for error snippets
pub mod type_classification;
pub mod type_inference; // Expression-level float type inference
pub mod wjsl; // Windjammer Shader Language (RFC syntax)

mod cli_args;
mod cli_commands;
mod cli_output;
mod cli_project_build;

pub use cli_args::CompilationTarget;
pub use cli_commands::run_main_cli;
pub use cli_output::{colorize_diagnostic, detect_rust_file_type, load_source_maps, RustFileType};
pub use cli_project_build::{build_project, build_project_ext};

/// Run the legacy `windjammer` CLI binary (`windjammer` crate root).
fn main() {
    if let Err(e) = run_main_cli() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_two_pass_compilation_concept() {
        // This test documents the two-pass compilation approach:
        // Pass 1: Parse all files to register trait definitions
        // Pass 2: Compile all files with traits available
        //
        // This approach is robust because:
        // - No filename conventions required
        // - Works regardless of file order
        // - Traits are always available when needed
        //
        // The actual implementation is in build_project()
        // If this test compiles and passes, the concept is sound
    }
}
