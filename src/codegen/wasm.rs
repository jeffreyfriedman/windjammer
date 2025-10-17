//! WebAssembly code generation backend
//!
//! This module wraps the existing WASM support to provide a clean backend interface

use super::backend::{CodegenBackend, CodegenConfig, CodegenOutput, Target};
use crate::analyzer::SignatureRegistry;
use crate::parser::Program;
use crate::CompilationTarget;
use anyhow::Result;

/// WebAssembly code generation backend
pub struct WasmBackend {
    // WASM-specific state
}

impl WasmBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for WasmBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CodegenBackend for WasmBackend {
    fn name(&self) -> &str {
        "WebAssembly"
    }

    fn target(&self) -> Target {
        Target::WebAssembly
    }

    fn generate(&self, program: &Program, config: &CodegenConfig) -> Result<CodegenOutput> {
        // Use existing CodeGenerator with WASM target
        let registry = SignatureRegistry::new();
        let mut generator = crate::codegen::CodeGenerator::new(registry, CompilationTarget::Wasm);

        // TODO: Pass analyzed functions once refactoring is complete
        let code = generator.generate_program(program, &[]);

        let mut output = CodegenOutput::new(code, "rs".to_string()); // WASM compiles through Rust

        // Generate additional WASM files
        if config.idiomatic_output {
            let html_test = self.generate_html_test_harness();
            output.add_file("index.html".to_string(), html_test);
        }

        Ok(output)
    }

    fn make_idiomatic(&self, code: String, config: &CodegenConfig) -> Result<String> {
        if !config.idiomatic_output {
            return Ok(code);
        }

        // Format with rustfmt (WASM target generates Rust code first)
        self.format_with_rustfmt(code)
    }

    fn generate_type_definitions(&self, _program: &Program) -> Option<String> {
        // Generate TypeScript definitions for JS bindings
        Some("// WebAssembly TypeScript bindings\n// TODO: Generate from exports\n".to_string())
    }

    fn generate_additional_files(
        &self,
        _program: &Program,
        _config: &CodegenConfig,
    ) -> Vec<(String, String)> {
        vec![
            ("Cargo.toml".to_string(), self.generate_cargo_toml()),
            ("index.html".to_string(), self.generate_html_test_harness()),
        ]
    }

    fn target_specific_optimizations(&self) -> Vec<String> {
        vec![
            "wasm_size_optimization".to_string(),
            "wasm_bindgen_optimization".to_string(),
        ]
    }
}

impl WasmBackend {
    fn format_with_rustfmt(&self, code: String) -> Result<String> {
        // Same as RustBackend
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
            Err(_) => return Ok(code),
        };

        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(code.as_bytes());
        }

        let output = child.wait_with_output()?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?)
        } else {
            Ok(code)
        }
    }

    fn generate_cargo_toml(&self) -> String {
        r#"[package]
name = "windjammer-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"

[profile.release]
opt-level = "z"  # Optimize for size
lto = true
"#
        .to_string()
    }

    fn generate_html_test_harness(&self) -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Windjammer WASM Test</title>
</head>
<body>
    <h1>Windjammer WASM</h1>
    <script type="module">
        import init from './pkg/windjammer_wasm.js';
        
        async function run() {
            await init();
            console.log("WASM loaded successfully!");
        }
        
        run();
    </script>
</body>
</html>
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_backend_creation() {
        let backend = WasmBackend::new();
        assert_eq!(backend.name(), "WebAssembly");
        assert_eq!(backend.target(), Target::WebAssembly);
    }

    #[test]
    fn test_generates_html_harness() {
        let backend = WasmBackend::new();
        let files = backend
            .generate_additional_files(&Program { items: vec![] }, &CodegenConfig::default());
        assert!(files.iter().any(|(name, _)| name == "index.html"));
    }
}
