//! Multi-target code generation
//!
//! This module provides code generation for multiple targets:
//! - Rust (default)
//! - JavaScript (ES2020+)
//! - WebAssembly
//!
//! ## Architecture
//!
//! The code generation is split into:
//! 1. **Backend trait** - Common interface for all targets
//! 2. **Target-specific backends** - Rust, JavaScript, WASM
//! 3. **Idiomatic formatters** - Ensure output is idiomatic for each language
//!
//! ## Backward Compatibility
//!
//! The existing `CodeGenerator` is preserved and re-exported for backward compatibility.
//! New code should use the `CodegenBackend` trait interface.

// New modular backends
pub mod backend;
pub mod javascript;
pub mod rust;
pub mod wasm;

// Re-export the CodeGenerator from the rust module
pub use rust::CodeGenerator;

use crate::parser::Program;
use anyhow::Result;
use backend::{CodegenConfig, CodegenOutput, Target};

/// High-level code generation function that dispatches to the appropriate backend
///
/// This is the recommended way to generate code in new code.
pub fn generate(
    program: &Program,
    target: Target,
    config: Option<CodegenConfig>,
) -> Result<CodegenOutput> {
    let config = config.unwrap_or_else(|| CodegenConfig {
        target,
        ..Default::default()
    });

    let backend = backend::create_backend(target);
    let mut output = backend.generate(program, &config)?;

    // Apply idiomatic formatting if requested
    if config.idiomatic_output {
        output.source = backend.make_idiomatic(output.source, &config)?;
    }

    // Generate type definitions if requested
    if config.type_definitions {
        if let Some(type_defs) = backend.generate_type_definitions(program) {
            output.type_definitions = Some(type_defs);
        }
    }

    // Generate additional files
    let additional = backend.generate_additional_files(program, &config);
    for (filename, content) in additional {
        output.add_file(filename, content);
    }

    Ok(output)
}

/// Generate Rust code (backward compatible function)
///
/// This maintains the existing API for code that uses the old interface.
pub fn generate_rust(program: &Program) -> Result<String> {
    let output = generate(program, Target::Rust, None)?;
    Ok(output.source)
}

/// Generate JavaScript code
pub fn generate_javascript(program: &Program) -> Result<CodegenOutput> {
    generate(program, Target::JavaScript, None)
}

/// Generate WebAssembly code
pub fn generate_wasm(program: &Program) -> Result<CodegenOutput> {
    generate(program, Target::WebAssembly, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_program() -> Program {
        Program { items: vec![] }
    }

    #[test]
    fn test_generate_rust() {
        let program = create_test_program();
        let result = generate_rust(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_javascript() {
        let program = create_test_program();
        let result = generate_javascript(&program);
        assert!(result.is_ok());

        if let Ok(output) = result {
            assert_eq!(output.extension, "js");
            assert!(output.source.contains("JavaScript"));
        }
    }

    #[test]
    fn test_generate_wasm() {
        let program = create_test_program();
        let result = generate_wasm(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_target_dispatch() {
        let program = create_test_program();

        let rust_output = generate(&program, Target::Rust, None);
        assert!(rust_output.is_ok());

        let js_output = generate(&program, Target::JavaScript, None);
        assert!(js_output.is_ok());

        let wasm_output = generate(&program, Target::WebAssembly, None);
        assert!(wasm_output.is_ok());
    }
}
