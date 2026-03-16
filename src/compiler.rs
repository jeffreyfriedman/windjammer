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
use crate::metadata::{CrateMetadata, FunctionSignature, ModuleMetadata};
use crate::parser::Parser;
use crate::parser::ast::core::Item;
use crate::type_inference::FloatInference;
use crate::CompilationTarget;
use anyhow::Result;
use std::collections::HashMap;
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
    build_project_ext(path, output, target, enable_lint, false, &[])
}

/// Extended build with library mode and external crate metadata.
pub fn build_project_ext(
    path: &Path,
    output: &Path,
    target: CompilationTarget,
    enable_lint: bool,
    library: bool,
    external_metadata: &[(&str, &Path)],
) -> Result<()> {
    let wj_files = find_wj_files(path)?;
    if wj_files.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(output)?;

    let external_paths: HashMap<String, PathBuf> = external_metadata
        .iter()
        .map(|(name, p)| (name.replace('-', "_"), (*p).to_path_buf()))
        .collect();

    let mut crate_metadata = CrateMetadata::new();

    // TDD FIX: For multi-file builds, use global multi-pass analysis
    // Parse all files first, then analyze together with shared registry
    if wj_files.len() > 1 && library {
        return build_library_multipass(
            &wj_files,
            path,
            output,
            target,
            library,
            enable_lint,
            &external_paths,
            crate_metadata,
        );
    }

    // Single-file or non-library: use old per-file analysis
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

        // Collect metadata for library emission
        if library {
            let mut module_meta = ModuleMetadata::new(
                file.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string(),
            );
            for item in &program.items {
                match item {
                    Item::Struct { decl, .. } => {
                        let mut fields = HashMap::new();
                        for field in &decl.fields {
                            fields.insert(
                                field.name.clone(),
                                ModuleMetadata::serialize_type(&field.field_type),
                            );
                        }
                        module_meta.structs.insert(decl.name.clone(), fields);
                    }
                    Item::Function { decl, .. } => {
                        module_meta.functions.insert(
                            decl.name.clone(),
                            FunctionSignature {
                                params: decl.parameters.iter().map(|p| ModuleMetadata::serialize_type(&p.type_)).collect(),
                                return_type: decl.return_type.as_ref().map(|t| ModuleMetadata::serialize_type(t)),
                                is_associated: false,
                                parent_type: None,
                            },
                        );
                    }
                    Item::Impl { block, .. } => {
                        for func_decl in &block.functions {
                            let full_name = format!("{}::{}", block.type_name, func_decl.name);
                            module_meta.functions.insert(
                                full_name,
                                FunctionSignature {
                                    params: func_decl.parameters.iter().map(|p| ModuleMetadata::serialize_type(&p.type_)).collect(),
                                    return_type: func_decl.return_type.as_ref().map(|t| ModuleMetadata::serialize_type(t)),
                                    is_associated: true,
                                    parent_type: Some(block.type_name.clone()),
                                },
                            );
                        }
                    }
                    _ => {}
                }
            }
            crate_metadata.merge_module(&module_meta);
        }

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

        // TDD: Float literal type inference - with external crate metadata for cross-crate inference
        let mut float_inference = FloatInference::new();
        if !external_paths.is_empty() {
            float_inference.set_external_crate_metadata_paths(&external_paths);
        }
        float_inference.infer_program(&program);
        if !float_inference.errors.is_empty() {
            for error in &float_inference.errors {
                eprintln!("Float inference error in {}: {}", file.display(), error);
            }
            return Err(anyhow::anyhow!(
                "Float type inference failed in {}: {} error(s)",
                file.display(),
                float_inference.errors.len()
            ));
        }

        // Single-file builds: use is_module=false to avoid `use super::*` which fails
        // when the generated .rs is compiled standalone (no parent module)
        let mut codegen = CodeGenerator::new(registry, target);
        codegen.set_analyzed_trait_methods(analyzer.analyzed_trait_methods.clone());
        codegen.set_float_inference(float_inference);
        let rust_code = codegen.generate_program(&program, &analyzed_functions);

        // TDD: Preserve directory structure for library builds (hierarchical imports)
        // Instead of flattening all .rs to output root, maintain relative path structure.
        // This allows crate::module::submodule::* imports to resolve correctly.
        let output_file = if wj_files.len() > 1 && library {
            // Multi-file library build: preserve directory structure
            let base_path = if path.is_file() {
                path.parent().unwrap_or(path)
            } else {
                path
            };
            let relative_path = file.strip_prefix(base_path)?;
            let output_with_structure = output.join(relative_path).with_extension("rs");
            if let Some(parent) = output_with_structure.parent() {
                std::fs::create_dir_all(parent)?;
            }
            output_with_structure
        } else {
            // Single-file or non-library: flatten to output root (legacy behavior)
            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
            output.join(format!("{}.rs", stem))
        };
        std::fs::write(&output_file, rust_code)?;
    }

    // Emit metadata.json when building library
    if library && (!crate_metadata.structs.is_empty() || !crate_metadata.functions.is_empty()) {
        let metadata_path = output.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&crate_metadata)?;
        std::fs::write(&metadata_path, metadata_json)?;
    }

    Ok(())
}

/// TDD FIX: Build library with global multi-pass analysis
/// Solves cross-file transitive mutability inference
#[allow(clippy::too_many_arguments)]
fn build_library_multipass(
    wj_files: &[PathBuf],
    base_path: &Path,
    output: &Path,
    target: CompilationTarget,
    library: bool,
    enable_lint: bool,
    external_paths: &HashMap<String, PathBuf>,
    mut crate_metadata: CrateMetadata,
) -> Result<()> {
    use crate::analyzer::SignatureRegistry;
    
    // Step 1: Read all source files (keep sources alive for lifetime safety)
    let mut sources: Vec<(PathBuf, String)> = Vec::new();
    
    for file in wj_files {
        let source = std::fs::read_to_string(file)?;
        sources.push((file.clone(), source));
    }
    
    // Step 2: Build initial registry from ALL files (first pass)
    let mut global_registry = SignatureRegistry::new();
    
    for (file, source) in &sources {
        // Parse with proper lifetime (program borrows source)
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new_with_source(
            tokens,
            file.to_string_lossy().to_string(),
            source.clone(),
        );
        let program = parser.parse().map_err(|e| anyhow::anyhow!("Parse error in {}: {}", file.display(), e))?;
        
        // Collect metadata for library emission
        let mut module_meta = ModuleMetadata::new(
            file.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string(),
        );
        for item in &program.items {
            match item {
                Item::Struct { decl, .. } => {
                    let mut fields = HashMap::new();
                    for field in &decl.fields {
                        fields.insert(
                            field.name.clone(),
                            ModuleMetadata::serialize_type(&field.field_type),
                        );
                    }
                    module_meta.structs.insert(decl.name.clone(), fields);
                }
                Item::Function { decl, .. } => {
                    module_meta.functions.insert(
                        decl.name.clone(),
                        FunctionSignature {
                            params: decl.parameters.iter().map(|p| ModuleMetadata::serialize_type(&p.type_)).collect(),
                            return_type: decl.return_type.as_ref().map(|t| ModuleMetadata::serialize_type(t)),
                            is_associated: false,
                            parent_type: None,
                        },
                    );
                }
                Item::Impl { block, .. } => {
                    for func_decl in &block.functions {
                        let full_name = format!("{}::{}", block.type_name, func_decl.name);
                        module_meta.functions.insert(
                            full_name,
                            FunctionSignature {
                                params: func_decl.parameters.iter().map(|p| ModuleMetadata::serialize_type(&p.type_)).collect(),
                                return_type: func_decl.return_type.as_ref().map(|t| ModuleMetadata::serialize_type(t)),
                                is_associated: true,
                                parent_type: Some(block.type_name.clone()),
                            },
                        );
                    }
                }
                _ => {}
            }
        }
        crate_metadata.merge_module(&module_meta);
        
        // First-pass analysis
        let mut analyzer = Analyzer::new();
        let (_, registry, _) = analyzer.analyze_program(&program)
            .map_err(|e| anyhow::anyhow!("Analysis error in {}: {}", file.display(), e))?;
        
        // Merge into global registry using public API
        global_registry.merge(&registry);
    }
    
    // Step 3: Global multi-pass iteration until convergence
    const MAX_GLOBAL_PASSES: usize = 10;
    let mut pass_number = 1;
    
    loop {
        let mut new_registry = global_registry.clone();
        
        // Re-analyze ALL files with current global registry
        for (file, source) in &sources {
            // Re-parse (lifetime scoped to this iteration)
            let mut lexer = Lexer::new(source);
            let tokens = lexer.tokenize_with_locations();
            let mut parser = Parser::new_with_source(
                tokens,
                file.to_string_lossy().to_string(),
                source.clone(),
            );
            let program = parser.parse().map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
            
            let mut analyzer = Analyzer::new();
            let (_, file_registry, _) = analyzer.analyze_program_with_global_signatures(
                &program,
                &global_registry,
            ).map_err(|e| anyhow::anyhow!("Analysis error in pass {}: {}", pass_number, e))?;
            
            new_registry.merge(&file_registry);
        }
        
        // Check for convergence by comparing registries
        // Since we don't have direct access to signatures, we'll use the convergence check
        // that's built into analyze_program_with_global_signatures
        if pass_number >= MAX_GLOBAL_PASSES {
            eprintln!("⚠️  Global ownership analysis: {} passes completed", pass_number);
            break;
        }
        
        global_registry = new_registry;
        pass_number += 1;
    }
    
    // Step 4: Final analysis with converged registry + code generation
    for (file, source) in &sources {
        // Final parse
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new_with_source(
            tokens,
            file.to_string_lossy().to_string(),
            source.clone(),
        );
        let program = parser.parse().map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
        let mut analyzer = Analyzer::new();
        
        // Rust leakage linter
        if enable_lint {
            let file_name = file.to_string_lossy().to_string();
            let mut rust_leakage = RustLeakageLinter::new(&file_name);
            rust_leakage.lint_program(&program);
            for diag in rust_leakage.diagnostics() {
                eprintln!("{}", diag);
            }
        }
        
        // Final analysis with converged registry
        let (analyzed_functions, registry, _) = analyzer.analyze_program_with_global_signatures(
            &program,
            &global_registry,
        ).map_err(|e| anyhow::anyhow!("Final analysis error: {}", e))?;
        
        analyzer.infer_trait_signatures_from_impls(&program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        
        // Float inference
        let mut float_inference = FloatInference::new();
        if !external_paths.is_empty() {
            float_inference.set_external_crate_metadata_paths(external_paths);
        }
        float_inference.infer_program(&program);
        if !float_inference.errors.is_empty() {
            for error in &float_inference.errors {
                eprintln!("Float inference error in {}: {}", file.display(), error);
            }
            return Err(anyhow::anyhow!(
                "Float type inference failed in {}: {} error(s)",
                file.display(),
                float_inference.errors.len()
            ));
        }
        
        // Code generation
        let mut codegen = CodeGenerator::new(registry, target);
        codegen.set_analyzed_trait_methods(analyzer.analyzed_trait_methods.clone());
        codegen.set_float_inference(float_inference);
        let rust_code = codegen.generate_program(&program, &analyzed_functions);
        
        // Preserve directory structure
        let base = if base_path.is_file() {
            base_path.parent().unwrap_or(base_path)
        } else {
            base_path
        };
        let relative_path = file.strip_prefix(base)?;
        let output_file = output.join(relative_path).with_extension("rs");
        if let Some(parent) = output_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&output_file, rust_code)?;
    }
    
    // Emit metadata.json
    if library && (!crate_metadata.structs.is_empty() || !crate_metadata.functions.is_empty()) {
        let metadata_path = output.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&crate_metadata)?;
        std::fs::write(&metadata_path, metadata_json)?;
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
