//! WebAssembly code generation backend
//!
//! This module wraps the existing WASM support to provide a clean backend interface

use super::backend::{CodegenBackend, CodegenConfig, CodegenOutput, Target};
use crate::analyzer::SignatureRegistry;
use crate::component_analyzer::ComponentAnalyzer;
use crate::parser::{Item, Program};
use crate::CompilationTarget;
use anyhow::Result;

/// WebAssembly code generation backend
pub struct WasmBackend {
    // WASM-specific state
    _component_analyzer: ComponentAnalyzer,
}

impl WasmBackend {
    pub fn new() -> Self {
        Self {
            _component_analyzer: ComponentAnalyzer::new(),
        }
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
        // Analyze components first
        let mut analyzer = ComponentAnalyzer::new();
        analyzer
            .analyze(&program.items)
            .map_err(|e| anyhow::anyhow!(e))?;

        // Check if this program has components
        let has_components = analyzer.all_components().next().is_some();

        if has_components {
            // Generate WASM component code
            self.generate_component_wasm(program, &analyzer, config)
        } else {
            // Use existing CodeGenerator with WASM target
            let registry = SignatureRegistry::new();
            let mut generator =
                crate::codegen::CodeGenerator::new(registry, CompilationTarget::Wasm);

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
    fn generate_component_wasm(
        &self,
        program: &Program,
        analyzer: &ComponentAnalyzer,
        _config: &CodegenConfig,
    ) -> Result<CodegenOutput> {
        use crate::codegen::wasm_component_generator::WasmComponentGenerator;
        use std::collections::HashMap;

        let mut code = String::new();

        // Generate imports
        code.push_str(&WasmComponentGenerator::generate_imports());

        // Collect component info
        let mut components = HashMap::new();
        for (name, info) in analyzer.all_components() {
            components.insert(name.clone(), info.clone());
        }

        let component_gen = WasmComponentGenerator::new(components);

        // Generate code for each item
        for item in &program.items {
            match item {
                Item::Struct {
                    decl: struct_decl, ..
                } => {
                    if analyzer.is_component(&struct_decl.name) {
                        code.push_str(&component_gen.generate_component_struct(struct_decl));
                    }
                }
                Item::Impl {
                    block: impl_block, ..
                } => {
                    if let Some(type_name) = self.extract_impl_type_name(impl_block) {
                        if let Some(component_info) = analyzer.get_component(&type_name) {
                            code.push_str(
                                &component_gen.generate_component_impl(impl_block, component_info),
                            );
                        }
                    }
                }
                _ => {
                    // Generate other items normally
                    // TODO: Use standard codegen for non-component items
                }
            }
        }

        let mut output = CodegenOutput::new(code, "rs".to_string());

        // Generate Cargo.toml for WASM component
        output.add_file(
            "Cargo.toml".to_string(),
            self.generate_component_cargo_toml(),
        );

        // Generate HTML harness
        output.add_file(
            "index.html".to_string(),
            self.generate_component_html_harness(),
        );

        Ok(output)
    }

    fn extract_impl_type_name(&self, impl_block: &crate::parser::ImplBlock) -> Option<String> {
        Some(impl_block.type_name.clone())
    }

    fn generate_component_cargo_toml(&self) -> String {
        r#"[package]
name = "windjammer-component"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = [
    "Document",
    "Element",
    "HtmlElement",
    "Node",
    "Text",
    "Window",
    "Event",
    "MouseEvent",
    "KeyboardEvent",
] }
js-sys = "0.3"
console_error_panic_hook = "0.1"
windjammer-runtime = { path = "../../../crates/windjammer-runtime", features = ["wasm"] }

[profile.release]
opt-level = "z"  # Optimize for size
lto = true
"#
        .to_string()
    }

    fn generate_component_html_harness(&self) -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Windjammer Component</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
        }
        .counter-app {
            text-align: center;
        }
        .counter-display {
            font-size: 48px;
            margin: 40px 0;
            font-weight: bold;
            color: #2563eb;
        }
        .counter-controls {
            display: flex;
            gap: 10px;
            justify-content: center;
        }
        .btn {
            padding: 15px 30px;
            font-size: 24px;
            border: 2px solid #ddd;
            border-radius: 8px;
            cursor: pointer;
            background: white;
            transition: all 0.2s;
        }
        .btn:hover {
            background: #f0f0f0;
            transform: scale(1.05);
        }
        .btn:active {
            transform: scale(0.95);
        }
        .btn-increment {
            color: #16a34a;
            border-color: #16a34a;
        }
        .btn-decrement {
            color: #dc2626;
            border-color: #dc2626;
        }
        .btn-reset {
            color: #6b7280;
        }
    </style>
</head>
<body>
    <div id="app"></div>
    
    <script type="module">
        import init, { Counter } from './pkg/windjammer_component.js';
        
        async function run() {
            await init();
            
            // Create and mount component
            const counter = new Counter();
            counter.mount();
            
            // Wire up event handlers
            document.addEventListener('click', (e) => {
                if (e.target.classList.contains('btn-increment')) {
                    counter.increment();
                } else if (e.target.classList.contains('btn-decrement')) {
                    counter.decrement();
                } else if (e.target.classList.contains('btn-reset')) {
                    counter.reset();
                }
            });
            
            console.log("✨ Windjammer component mounted!");
            console.log("🎯 Click the buttons to interact!");
        }
        
        run().catch(console.error);
    </script>
</body>
</html>
"#
        .to_string()
    }

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
