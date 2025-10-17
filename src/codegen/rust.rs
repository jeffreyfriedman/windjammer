//! Rust code generation backend
//!
//! This module wraps the existing `CodeGenerator` to provide a clean backend interface
//! while maintaining 100% backward compatibility.

use super::backend::{CodegenBackend, CodegenConfig, CodegenOutput, Target};
use crate::analyzer::SignatureRegistry;
use crate::parser::Program;
use crate::CompilationTarget;
use anyhow::Result;

/// Rust code generation backend
pub struct RustBackend {
    // Future: Move CodeGenerator internals here
}

impl RustBackend {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate idiomatic Rust code using rustfmt
    fn format_with_rustfmt(&self, code: String) -> Result<String> {
        // Try to format with rustfmt if available
        use std::io::Write;
        use std::process::{Command, Stdio};

        let mut child = match Command::new("rustfmt")
            .arg("--edition=2021")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => {
                // rustfmt not available, return unformatted
                return Ok(code);
            }
        };

        // Write code to stdin
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(code.as_bytes());
        }

        // Read formatted output
        let output = child.wait_with_output()?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?)
        } else {
            // Format failed, return original
            Ok(code)
        }
    }
}

impl Default for RustBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CodegenBackend for RustBackend {
    fn name(&self) -> &str {
        "Rust"
    }

    fn target(&self) -> Target {
        Target::Rust
    }

    fn generate(&self, program: &Program, config: &CodegenConfig) -> Result<CodegenOutput> {
        // Use existing CodeGenerator for now
        let registry = SignatureRegistry::new();
        let target = match config.target {
            Target::WebAssembly => CompilationTarget::Wasm,
            _ => CompilationTarget::Wasm, // Default to Wasm for now
        };

        let mut generator = crate::codegen::CodeGenerator::new(registry, target);

        // TODO: Pass analyzed functions once refactoring is complete
        let code = generator.generate_program(program, &[]);

        let mut output = CodegenOutput::new(code, "rs".to_string());

        // Generate Cargo.toml if configured
        if !config.output_dir.as_os_str().is_empty() {
            let cargo_toml = self.generate_cargo_toml(program);
            output.add_file("Cargo.toml".to_string(), cargo_toml);
        }

        Ok(output)
    }

    fn make_idiomatic(&self, code: String, config: &CodegenConfig) -> Result<String> {
        if !config.idiomatic_output {
            return Ok(code);
        }

        // Format with rustfmt
        self.format_with_rustfmt(code)
    }

    fn generate_additional_files(
        &self,
        program: &Program,
        _config: &CodegenConfig,
    ) -> Vec<(String, String)> {
        vec![("Cargo.toml".to_string(), self.generate_cargo_toml(program))]
    }

    fn target_specific_optimizations(&self) -> Vec<String> {
        vec![
            "defer_drop".to_string(),
            "inline_hints".to_string(),
            "escape_analysis".to_string(),
            "simd_vectorization".to_string(),
        ]
    }
}

impl RustBackend {
    fn generate_cargo_toml(&self, _program: &Program) -> String {
        // Basic Cargo.toml template
        r#"[package]
name = "windjammer-generated"
version = "0.1.0"
edition = "2021"

[dependencies]
# Add dependencies based on program requirements
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_backend_creation() {
        let backend = RustBackend::new();
        assert_eq!(backend.name(), "Rust");
        assert_eq!(backend.target(), Target::Rust);
    }

    #[test]
    fn test_target_specific_optimizations() {
        let backend = RustBackend::new();
        let opts = backend.target_specific_optimizations();
        assert!(opts.contains(&"defer_drop".to_string()));
        assert!(opts.contains(&"escape_analysis".to_string()));
    }
}
