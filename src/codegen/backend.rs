//! Code generation backend trait for multi-target compilation
//!
//! This module defines the common interface that all code generation backends
//! (Rust, JavaScript, WASM) must implement. This architecture ensures:
//!
//! 1. Clean separation between targets
//! 2. Shared optimizations benefit all targets
//! 3. Target-specific idioms and optimizations
//! 4. Easy to add new targets

use crate::parser::Program;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Compilation target
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    /// Rust source code (default)
    Rust,
    /// Go source code (experimental)
    Go,
    /// JavaScript (ES2020+)
    JavaScript,
    /// WebAssembly
    WebAssembly,
}

impl std::str::FromStr for Target {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Ok(Target::Rust),
            "go" | "golang" => Ok(Target::Go),
            "javascript" | "js" => Ok(Target::JavaScript),
            "wasm" | "webassembly" => Ok(Target::WebAssembly),
            _ => Err(format!("Unknown target: {}", s)),
        }
    }
}

impl Target {
    pub fn as_str(&self) -> &'static str {
        match self {
            Target::Rust => "rust",
            Target::Go => "go",
            Target::JavaScript => "javascript",
            Target::WebAssembly => "webassembly",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Target::Rust => "rs",
            Target::Go => "go",
            Target::JavaScript => "js",
            Target::WebAssembly => "wasm",
        }
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Code generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodegenConfig {
    /// Target platform
    pub target: Target,

    /// Output directory
    pub output_dir: PathBuf,

    /// Generate source maps (for debugging)
    pub source_maps: bool,

    /// Generate type definitions (.d.ts for JS, etc.)
    pub type_definitions: bool,

    /// Optimization level (0 = none, 1 = basic, 2 = aggressive)
    pub optimization_level: u8,

    /// Generate idiomatic code for the target language
    pub idiomatic_output: bool,

    /// Pretty-print output (vs minified)
    pub pretty_print: bool,

    /// Include comments in generated code
    pub include_comments: bool,

    /// Minify output (JS target)
    pub minify: bool,

    /// Enable tree shaking (dead code elimination)
    pub tree_shake: bool,

    /// Include polyfills for older browsers (JS target)
    pub polyfills: bool,

    /// Apply V8-specific optimizations (JS target)
    pub v8_optimize: bool,
}

impl Default for CodegenConfig {
    fn default() -> Self {
        Self {
            target: Target::Rust,
            output_dir: PathBuf::from("output"),
            source_maps: true,
            type_definitions: true,
            optimization_level: 2,
            idiomatic_output: true,
            pretty_print: true,
            include_comments: false,
            minify: false,
            tree_shake: false,
            polyfills: false,
            v8_optimize: false,
        }
    }
}

/// Output from code generation
#[derive(Debug, Clone)]
pub struct CodegenOutput {
    /// Primary source file content
    pub source: String,

    /// Source map (if generated)
    pub source_map: Option<String>,

    /// Type definitions (if generated)
    pub type_definitions: Option<String>,

    /// Additional files (manifests, configs, etc.)
    pub additional_files: Vec<(String, String)>, // (filename, content)

    /// File extension for the primary source
    pub extension: String,
}

impl CodegenOutput {
    pub fn new(source: String, extension: String) -> Self {
        Self {
            source,
            source_map: None,
            type_definitions: None,
            additional_files: Vec::new(),
            extension,
        }
    }

    pub fn with_source_map(mut self, source_map: String) -> Self {
        self.source_map = Some(source_map);
        self
    }

    pub fn with_type_definitions(mut self, type_defs: String) -> Self {
        self.type_definitions = Some(type_defs);
        self
    }

    pub fn add_file(&mut self, filename: String, content: String) {
        self.additional_files.push((filename, content));
    }
}

/// Code generation backend trait
///
/// Each target (Rust, JavaScript, WASM) implements this trait to provide
/// target-specific code generation while sharing common infrastructure.
pub trait CodegenBackend: Send + Sync {
    /// Target name (e.g., "Rust", "JavaScript")
    fn name(&self) -> &str;

    /// Target identifier
    fn target(&self) -> Target;

    /// Generate code from the AST
    fn generate(&self, program: &Program, config: &CodegenConfig) -> Result<CodegenOutput>;

    /// Apply target-specific idioms and formatting
    ///
    /// This is called after code generation to ensure output is idiomatic
    /// for the target language. Examples:
    /// - Rust: Use `rustfmt`
    /// - JavaScript: Use Prettier-like formatting, convert to idiomatic patterns
    /// - WASM: Optimize for size/performance
    fn make_idiomatic(&self, code: String, _config: &CodegenConfig) -> Result<String> {
        // Default: no transformation
        Ok(code)
    }

    /// Generate type definitions for the target (if applicable)
    ///
    /// Examples:
    /// - JavaScript: Generate .d.ts TypeScript definitions
    /// - Rust: Generate doc comments
    /// - WASM: Generate JS bindings with types
    fn generate_type_definitions(&self, _program: &Program) -> Option<String> {
        None
    }

    /// Generate additional files (manifests, configs, etc.)
    ///
    /// Examples:
    /// - Rust: Generate Cargo.toml
    /// - JavaScript: Generate package.json
    /// - WASM: Generate HTML test harness
    fn generate_additional_files(
        &self,
        _program: &Program,
        _config: &CodegenConfig,
    ) -> Vec<(String, String)> {
        Vec::new()
    }

    /// Target-specific optimization passes
    ///
    /// These run AFTER target-agnostic optimizations
    fn target_specific_optimizations(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Factory function to create the appropriate backend for a target
pub fn create_backend(target: Target) -> Box<dyn CodegenBackend> {
    match target {
        Target::Rust => Box::new(crate::codegen::rust::RustBackend::new()),
        Target::Go => Box::new(crate::codegen::go::GoBackend::new()),
        Target::JavaScript => Box::new(crate::codegen::javascript::JavaScriptBackend::new()),
        Target::WebAssembly => Box::new(crate::codegen::wasm::WasmBackend::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_from_str() {
        use std::str::FromStr;

        assert_eq!(Target::from_str("rust"), Ok(Target::Rust));
        assert_eq!(Target::from_str("Rust"), Ok(Target::Rust));
        assert_eq!(Target::from_str("RS"), Ok(Target::Rust));

        assert_eq!(Target::from_str("go"), Ok(Target::Go));
        assert_eq!(Target::from_str("golang"), Ok(Target::Go));

        assert_eq!(Target::from_str("javascript"), Ok(Target::JavaScript));
        assert_eq!(Target::from_str("JS"), Ok(Target::JavaScript));

        assert_eq!(Target::from_str("wasm"), Ok(Target::WebAssembly));
        assert_eq!(Target::from_str("webassembly"), Ok(Target::WebAssembly));

        assert!(Target::from_str("unknown").is_err());
    }

    #[test]
    fn test_target_extensions() {
        assert_eq!(Target::Rust.file_extension(), "rs");
        assert_eq!(Target::Go.file_extension(), "go");
        assert_eq!(Target::JavaScript.file_extension(), "js");
        assert_eq!(Target::WebAssembly.file_extension(), "wasm");
    }

    #[test]
    fn test_codegen_config_defaults() {
        let config = CodegenConfig::default();
        assert_eq!(config.target, Target::Rust);
        assert!(config.idiomatic_output);
        assert!(config.pretty_print);
        assert_eq!(config.optimization_level, 2);
    }
}
