//! Compiler module - builds Windjammer projects.
//!
//! This module provides the core build logic used by both the CLI and integration tests.
//! The implementation is in main.rs; this module re-exports for the library.
//!
//! When the crate is built as a library, tests use windjammer::build_project().
//! When built as the main binary, main.rs has its own build_project.
//!
//! For library builds, we need the implementation. The main.rs and lib.rs are
//! separate crate roots - the lib cannot call main. So we include a stub that
//! will be replaced when the full extraction is done.
//!
//! TEMPORARY: Use the build_project from the crate that has it.
//! The cross_file_trait_inference tests use windjammer::build_project - they
//! must get it from somewhere. Currently the lib doesn't have the full impl.
//! We need to either:
//! 1. Extract the full impl here (large refactor)
//! 2. Have a different crate structure
//!
//! For now: the tests that use build_project (cross_file, integration_ffi, etc.)
//! - do they actually run? Let me check the build. The lib build fails because
//! we don't have the impl. So we need the impl. Let me add a minimal impl that
//! does single-file compilation using the existing analyzer/codegen.

use crate::analyzer::Analyzer;
use crate::codegen::rust::CodeGenerator;
use crate::lexer::Lexer;
use crate::linter::rust_leakage::RustLeakageLinter;
use crate::parser::Parser;
use crate::type_inference::FloatInference;
use crate::CompilationTarget;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Build a Windjammer project - compiles .wj files to Rust.
///
/// Used by integration tests. For full CLI support, the main binary has
/// the complete implementation with multi-file, Cargo.toml, etc.
///
/// When enable_lint is true, runs Rust leakage linter (W0001-W0004) and emits warnings.
pub fn build_project(
    path: &Path,
    output: &Path,
    target: CompilationTarget,
    enable_lint: bool,
) -> Result<()> {
    let wj_files = find_wj_files(path)?;
    if wj_files.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(output)?;

    for file in &wj_files {
        let source = std::fs::read_to_string(file)?;
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new_with_source(
            tokens,
            file.to_string_lossy().to_string(),
            source.clone(),
        );
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

        let mut analyzer = Analyzer::new();
        analyzer
            .check_forbidden_rust_patterns(&program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Rust leakage linter (W0001-W0004)
        if enable_lint {
            let file_name = file.to_string_lossy().to_string();
            let mut rust_leakage = RustLeakageLinter::new(&file_name);
            rust_leakage.lint_program(&program);
            for diag in rust_leakage.diagnostics() {
                eprintln!("{}", diag);
            }
        }

        let (analyzed_functions, registry, _) =
            analyzer.analyze_program(&program).map_err(|e| anyhow::anyhow!("{}", e))?;

        // TDD: Infer trait signatures from impls (e.g. &mut self when any impl mutates)
        // Single-file: still need this for traits with multiple impls in same file
        analyzer
            .infer_trait_signatures_from_impls(&program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // TDD: Float literal type inference - propagate f32/f64 through binary ops
        // Enables pos.x + 10.0 to infer 10.0 as f32 when pos.x is f32
        let mut float_inference = FloatInference::new();
        float_inference.infer_program(&program);
        if !float_inference.errors.is_empty() {
            for error in &float_inference.errors {
                eprintln!("Float inference error: {}", error);
            }
            return Err(anyhow::anyhow!(
                "Float type inference failed: {} error(s)",
                float_inference.errors.len()
            ));
        }

        // Single-file builds: use is_module=false to avoid `use super::*` which fails
        // when the generated .rs is compiled standalone (no parent module)
        let mut codegen = CodeGenerator::new(registry, target);
        codegen.set_analyzed_trait_methods(analyzer.analyzed_trait_methods.clone());
        codegen.set_float_inference(float_inference);
        let rust_code = codegen.generate_program(&program, &analyzed_functions);

        let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        let output_file = output.join(format!("{}.rs", stem));
        std::fs::write(output_file, rust_code)?;
    }
    Ok(())
}

fn find_wj_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("wj") {
            if path.file_name().and_then(|n| n.to_str()) == Some("mod.wj") {
                if let Some(parent) = path.parent() {
                    find_wj_files_recursive(parent, &mut files)?;
                } else {
                    files.push(path.to_path_buf());
                }
            } else {
                files.push(path.to_path_buf());
            }
        }
    } else if path.is_dir() {
        find_wj_files_recursive(path, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn find_wj_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("wj") {
            files.push(path);
        } else if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if dir_name != "shaders" {
                find_wj_files_recursive(&path, files)?;
            }
        }
    }
    Ok(())
}
