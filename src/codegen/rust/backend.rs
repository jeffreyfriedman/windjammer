//! Rust code generation backend
//!
//! This module wraps the existing `CodeGenerator` to provide a clean backend interface
//! while maintaining 100% backward compatibility.

use crate::analyzer::SignatureRegistry;
use crate::codegen::backend::{CodegenBackend, CodegenConfig, CodegenOutput, Target};
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

    /// Extract external crate dependencies from generated Rust code
    /// 
    /// This scans the generated code for `use` statements and identifies external crates.
    /// Built-in crates like `std`, `core`, and `alloc` are excluded.
    pub fn extract_external_dependencies(code: &str) -> Vec<String> {
        use std::collections::HashSet;
        
        let mut deps = HashSet::new();
        let builtin_crates = ["std", "core", "alloc"];
        
        // Simple parser: match `use crate_name::...` patterns
        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("use ") {
                // Extract the first identifier after "use "
                if let Some(rest) = trimmed.strip_prefix("use ") {
                    if let Some(first_ident_end) = rest.find("::").or_else(|| rest.find(";")) {
                        let crate_name = &rest[..first_ident_end];
                        
                        // Skip builtin crates
                        if !builtin_crates.contains(&crate_name) {
                            deps.insert(crate_name.to_string());
                        }
                    }
                }
            }
        }
        
        let mut result: Vec<String> = deps.into_iter().collect();
        result.sort(); // Deterministic ordering
        result
    }

    /// Generate Cargo.toml with proper dependencies based on generated code
    pub fn generate_cargo_toml_with_code(&self, code: &str) -> String {
        let deps = Self::extract_external_dependencies(code);
        
        let mut cargo_toml = String::from(
            r#"[package]
name = "windjammer-generated"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
        );
        
        // Add detected dependencies with path references
        for dep in deps {
            let dep_line = match dep.as_str() {
                "windjammer_game_core" => {
                    "windjammer-game-core = { path = \"../windjammer-game/windjammer-game-core\" }\n".to_string()
                }
                "windjammer_runtime" => {
                    "windjammer-runtime = { path = \"../windjammer/crates/windjammer-runtime\" }\n".to_string()
                }
                _ => {
                    // Unknown external crate, add as crates.io dependency
                    format!("{} = \"*\"\n", dep)
                }
            };
            cargo_toml.push_str(&dep_line);
        }
        
        cargo_toml
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
        // TDD FIX: Analyze program first to populate signature registry!
        // This was the root cause of the auto-clone bug - method signatures weren't available
        let target = match config.target {
            Target::WebAssembly => CompilationTarget::Wasm,
            _ => CompilationTarget::Wasm, // Default to Wasm for now
        };
        
        // Run analyzer to get signatures and analyzed functions
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, signatures, analyzed_trait_methods) = analyzer
            .analyze_program(program)
            .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;

        let mut generator = crate::codegen::CodeGenerator::new(signatures, target);
        generator.set_analyzed_trait_methods(analyzed_trait_methods);
        
        // Pass analyzed functions so codegen has ownership info
        let code = generator.generate_program(program, &analyzed);

        let mut output = CodegenOutput::new(code.clone(), "rs".to_string());

        // Generate Cargo.toml if configured
        if !config.output_dir.as_os_str().is_empty() {
            // NEW: Track dependencies from generated code
            let cargo_toml = self.generate_cargo_toml_with_code(&code);
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
        // TDD FIX: Analyze program first (same as generate())
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, signatures, analyzed_trait_methods) = match analyzer.analyze_program(program) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Warning: Analysis error in generate_additional_files: {}", e);
                (vec![], SignatureRegistry::new(), std::collections::HashMap::new())
            }
        };
        
        let target = CompilationTarget::Wasm; // Default target
        let mut generator = crate::codegen::CodeGenerator::new(signatures, target);
        generator.set_analyzed_trait_methods(analyzed_trait_methods);
        let code = generator.generate_program(program, &analyzed);
        
        vec![("Cargo.toml".to_string(), self.generate_cargo_toml_with_code(&code))]
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
    // Deprecated: Use generate_cargo_toml_with_code() instead
    // This is kept for now to avoid breaking changes, but is no longer used
    #[allow(dead_code)]
    fn generate_cargo_toml(&self, _program: &Program) -> String {
        // This method is deprecated. The new implementation tracks dependencies
        // from the generated code via generate_cargo_toml_with_code()
        self.generate_cargo_toml_with_code("")
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

    #[test]
    fn test_dependency_tracking_extracts_external_crates() {
        // RED: This test should fail until we implement dependency tracking
        let generated_code = r#"
use windjammer_game_core::prelude::*;
use windjammer_runtime::test::*;
use std::collections::HashMap;

fn main() {
    println!("Hello");
}
"#;
        
        let deps = RustBackend::extract_external_dependencies(generated_code);
        
        // Should extract external crates, but NOT std
        assert!(deps.contains(&"windjammer_game_core".to_string()), 
                "Should extract windjammer_game_core");
        assert!(deps.contains(&"windjammer_runtime".to_string()), 
                "Should extract windjammer_runtime");
        assert!(!deps.contains(&"std".to_string()), 
                "Should NOT extract std (builtin)");
        assert_eq!(deps.len(), 2, "Should have exactly 2 external dependencies");
    }

    #[test]
    fn test_cargo_toml_includes_tracked_dependencies() {
        // GREEN: Test that Cargo.toml includes dependencies extracted from code
        let backend = RustBackend::new();
        let generated_code = r#"
use windjammer_game_core::math::Vec3;
use windjammer_runtime::test::assert_eq;

fn test() {}
"#;
        
        let cargo_toml = backend.generate_cargo_toml_with_code(generated_code);
        
        // Cargo.toml uses hyphens (windjammer-game-core) not underscores
        assert!(cargo_toml.contains("windjammer-game-core"), 
                "Cargo.toml should include windjammer-game-core (with hyphens)");
        assert!(cargo_toml.contains("windjammer-runtime"), 
                "Cargo.toml should include windjammer-runtime (with hyphens)");
        assert!(cargo_toml.contains("path = "), 
                "Should use path references for local crates");
        
        // Verify the actual format
        assert!(cargo_toml.contains("windjammer-game-core = { path = "), 
                "Should have proper dependency declaration for game-core");
        assert!(cargo_toml.contains("windjammer-runtime = { path = "), 
                "Should have proper dependency declaration for runtime");
    }
}
