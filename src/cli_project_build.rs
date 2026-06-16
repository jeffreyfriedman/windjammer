//! Multipass project build for the legacy `windjammer` binary crate.

use crate::analyzer;
use crate::build_utils;
use crate::cargo_integration;
use crate::codegen::rust::backend::RustBackend;
use crate::config;
use crate::file_compiler;
use crate::lexer;
use crate::parser;
use crate::parser_impl;
use crate::project_paths;

use crate::cli_args::CompilationTarget;
use anyhow::Result;
use colored::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Quick Copy type check for PASS 0 (no full analyzer available).
/// Checks if a type is Copy based on primitives and already-known Copy structs/enums.
fn is_type_copy_quick(
    ty: &parser::Type,
    copy_structs: &HashSet<String>,
    copy_enums: &HashSet<String>,
) -> bool {
    crate::type_classification::is_type_copy_with_registries(ty, copy_structs, copy_enums)
}

/// Extended build with library mode and external crate metadata.
/// Used by CLI when --library or --metadata is passed.
/// The full main.rs build_project doesn't yet support these - delegate to compiler for simple builds.
pub fn build_project_ext(
    path: &Path,
    output: &Path,
    target: CompilationTarget,
    enable_lint: bool,
    library: bool,
    external_metadata: &[(&str, &Path)],
) -> Result<()> {
    // Library builds and cross-crate metadata require the multipass compiler pipeline.
    if !external_metadata.is_empty() || library {
        return crate::compiler::build_project_ext(
            path,
            output,
            target,
            enable_lint,
            library,
            external_metadata,
        );
    }
    // Full multi-file build
    build_project(path, output, target, enable_lint)
}

pub fn build_project(
    path: &Path,
    output: &Path,
    target: CompilationTarget,
    enable_lint: bool,
) -> Result<()> {
    println!(
        "{} Windjammer files in: {:?}",
        "Building".green().bold(),
        path
    );
    println!("Target: {:?}", target);

    // Find all .wj files
    let wj_files = find_wj_files(path)?;

    if wj_files.is_empty() {
        println!("{} No .wj files found", "Warning:".yellow().bold());
        return Ok(());
    }

    println!("Found {} file(s)", wj_files.len());

    // Create output directory
    std::fs::create_dir_all(output)?;

    let mut has_errors = false;
    let mut all_stdlib_modules = HashSet::new();
    let mut all_external_crates = Vec::new();

    // Create a single ModuleCompiler for all files to share trait registry
    let mut module_compiler = file_compiler::ModuleCompiler::new(target, enable_lint);

    // Load windjammer.toml if it exists (search up directory tree)
    let mut search_dir = if path.is_file() {
        path.parent().unwrap_or(Path::new("."))
    } else {
        path
    };

    let mut config_loaded = false;
    for _ in 0..5 {
        let config_path = search_dir.join("windjammer.toml");
        if config_path.exists() {
            match config::WjConfig::load_from_file(&config_path) {
                Ok(config) => {
                    // Add configured source roots
                    if let Some(sources) = &config.sources {
                        for root in &sources.roots {
                            // Resolve path relative to config file directory
                            let root_path = search_dir.join(root);
                            if root_path.exists() && root_path.is_dir() {
                                module_compiler.add_source_root(root_path);
                            } else {
                                eprintln!(
                                    "Warning: Source root not found: {}",
                                    root_path.display()
                                );
                            }
                        }
                        config_loaded = true;
                    }
                    break;
                }
                Err(e) => {
                    eprintln!("Warning: Failed to load {}: {}", config_path.display(), e);
                }
            }
        }

        // Go up one directory
        if let Some(parent) = search_dir.parent() {
            search_dir = parent;
        } else {
            break;
        }
    }

    if !config_loaded {
        eprintln!("Note: No windjammer.toml found. Module imports will only work with relative paths (./module) or std:: prefix.");
    }

    // Add the project's own source directory to source_roots
    // This ensures that internal modules (vec2, camera2d, etc.) are not treated as external crates
    let project_source_dir = if path.is_file() {
        path.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        path.to_path_buf()
    };
    module_compiler.add_source_root(project_source_dir);

    // PASS 0: Quick parse all files to register Copy structs
    // This ensures Copy type detection works across all files.
    // WINDJAMMER PHILOSOPHY: Auto-detect Copy structs when all fields are primitive/Copy.
    // We do multiple iterations to handle structs that reference other Copy structs.
    let mut global_copy_structs = HashSet::new();

    // Collect all struct declarations for iterative Copy detection
    struct StructInfo {
        name: String,
        field_types: Vec<parser::Type>,
    }
    let mut all_structs: Vec<StructInfo> = Vec::new();
    // Also collect fieldless enums (always Copy)
    let mut copy_enums: HashSet<String> = HashSet::new();

    for file in &wj_files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let mut lexer = lexer::Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = parser_impl::Parser::new(tokens);

        if let Ok(program) = parser.parse() {
            for item in &program.items {
                match item {
                    parser::Item::Struct { decl, .. } => {
                        let has_copy = decl.decorators.iter().any(|d| {
                            d.name == "derive"
                                && d.arguments.iter().any(|(_, arg)| {
                                    if let parser::Expression::Identifier { name, .. } = arg {
                                        name == "Copy"
                                    } else {
                                        false
                                    }
                                })
                        });
                        let field_types: Vec<parser::Type> =
                            decl.fields.iter().map(|f| f.field_type.clone()).collect();
                        all_structs.push(StructInfo {
                            name: decl.name.clone(),
                            field_types,
                        });
                        if has_copy {
                            global_copy_structs.insert(decl.name.clone());
                        }
                        // CROSS-MODULE STRUCT FIELD TYPES: Collect field types for type inference
                        // Enables CodeGenerator to resolve field types on imported structs,
                        // preventing unnecessary .clone() on Copy-type fields
                        let mut struct_fields = HashMap::new();
                        for field in &decl.fields {
                            struct_fields.insert(field.name.clone(), field.field_type.clone());
                        }
                        module_compiler
                            .global_struct_field_types
                            .insert(decl.name.clone(), struct_fields);
                    }
                    parser::Item::Enum { decl, .. } => {
                        use crate::parser::ast::EnumVariantData;
                        let is_unit_only = decl
                            .variants
                            .iter()
                            .all(|v| matches!(v.data, EnumVariantData::Unit));
                        if is_unit_only {
                            copy_enums.insert(decl.name.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // TDD FIX: When multiple structs share the same name (different modules/files),
    // be CONSERVATIVE: only mark as Copy if ALL definitions are Copy.
    // Otherwise one Copy-able GameState in file A poisons non-Copy GameState in file B,
    // causing E0382 errors when passing game_state to multiple functions.
    //
    // Strategy: Group structs by name, check if ALL variants are Copy, only then add to registry.
    let mut structs_by_name: HashMap<String, Vec<&StructInfo>> = HashMap::new();
    for s in &all_structs {
        structs_by_name.entry(s.name.clone()).or_default().push(s);
    }

    // Fixed-point iteration: keep discovering Copy structs until stable
    loop {
        let mut changed = false;
        for (name, variants) in &structs_by_name {
            if global_copy_structs.contains(name) {
                continue; // Already known Copy
            }

            // Empty structs are Copy in Rust (and codegen derives Copy); include them here
            // so analyzer/registry match codegen and avoid E0382 from wrong `self` inference.
            //
            // Check if ALL variants of this struct name are Copy
            let all_variants_copy = variants.iter().all(|s| {
                s.field_types.is_empty()
                    || s.field_types
                        .iter()
                        .all(|ty| is_type_copy_quick(ty, &global_copy_structs, &copy_enums))
            });

            if all_variants_copy {
                global_copy_structs.insert(name.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Transfer Copy structs AND enums to ModuleCompiler's registry
    // Both user-defined Copy structs and unit-only enums (always Copy) must be included
    // so the CodeGenerator can suppress unnecessary .clone() on these types.
    module_compiler.copy_structs_registry = global_copy_structs;
    module_compiler.copy_structs_registry.extend(copy_enums);

    // PASS 1: Quick parse all files to register trait definitions
    // This ensures all traits are available before any file compilation
    for file in &wj_files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => continue, // Skip files we can't read
        };

        // Quick parse to find trait definitions
        let mut lexer = lexer::Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = parser_impl::Parser::new(tokens);

        if let Ok(program) = parser.parse() {
            // LANGUAGE DESIGN CHECK: Prohibit Rust-specific patterns (.as_str())
            // Check immediately after parsing, before trait registration
            let checker_analyzer = analyzer::Analyzer::new();
            if let Err(e) = checker_analyzer.check_forbidden_rust_patterns(&program) {
                eprintln!("{}", e);
                anyhow::bail!("{}", e);
            }

            // Register any trait definitions found
            let mut has_traits = false;
            for item in &program.items {
                if let parser::Item::Trait { decl, .. } = item {
                    module_compiler
                        .trait_registry
                        .insert(decl.name.clone(), decl.clone());
                    has_traits = true;
                }
            }
            // ARENA FIX: Keep parser alive if we stored trait definitions
            if has_traits {
                module_compiler._trait_parsers.push(parser);
            }
        }
    }

    // Determine source_root: if path is a file, find the actual source root
    // BUGFIX: For nested files like src/ecs/entity.wj, we need to find src,
    // not just the immediate parent (src/ecs)
    let source_root = if path.is_file() {
        project_paths::find_source_root(path)
            .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")))
    } else {
        path
    };

    // PASS 2: Full compilation with all traits available
    let is_multi_file = wj_files.len() > 1;
    println!("DEBUG: Starting PASS 2 with {} files", wj_files.len());
    for file in &wj_files {
        let file_name = file.file_name().unwrap().to_str().unwrap();

        print!("  Compiling {:?}... ", file_name);

        match file_compiler::compile_file_with_compiler(
            source_root,
            file,
            output,
            &mut module_compiler,
            is_multi_file,
            true,
        ) {
            Ok((stdlib_modules, external_crates)) => {
                println!("{}", "✓".green());
                all_stdlib_modules.extend(stdlib_modules);
                all_external_crates.extend(external_crates);
            }
            Err(e) => {
                println!("{}", "✗".red());
                println!("    Error: {}", e);
                has_errors = true;
            }
        }
    }

    if !has_errors {
        // THE WINDJAMMER WAY: Finalize trait inference across ALL files
        // This ensures trait signatures are inferred from ALL implementations in the project
        println!(
            "{}",
            "Analyzing trait signatures across all files...".cyan()
        );
        println!(
            "  Total programs collected: {}",
            module_compiler.all_programs.len()
        );
        if let Err(e) = module_compiler.finalize_trait_inference() {
            println!("{}", "✗ Trait inference failed".red());
            println!("    Error: {}", e);
            anyhow::bail!("Trait inference failed: {}", e);
        }
        println!("{}", "✓ Trait inference complete".green());

        // THE WINDJAMMER WAY: Regenerate ALL files with inferred trait signatures
        // Both trait definitions AND implementations need to be regenerated
        // with the updated cross-file inferred signatures
        println!(
            "{}",
            "Regenerating with inferred trait signatures...".cyan()
        );
        for file in &wj_files {
            let file_name = file.file_name().unwrap().to_str().unwrap();

            print!("  Updating {:?}... ", file_name);
            match file_compiler::compile_file_with_compiler(
                source_root,
                file,
                output,
                &mut module_compiler,
                is_multi_file,
                false,
            ) {
                Ok(_) => println!("{}", "✓".green()),
                Err(e) => {
                    println!("{}", "✗".red());
                    println!("    Error: {}", e);
                    has_errors = true;
                }
            }
        }

        if has_errors {
            println!("\n{} Trait regeneration failed", "Error:".red().bold());
            anyhow::bail!("Trait regeneration failed");
        }

        // THE WINDJAMMER WAY: Generate lib.rs FIRST (before Cargo.toml) for multi-file projects
        // This allows Cargo.toml generation to detect lib.rs and create [lib] instead of [[bin]]
        // A project is multi-file if:
        // 1. Multiple .wj files were found, OR
        // 2. The input is a mod.wj file (implies multi-file structure)
        let is_root_mod_wj = wj_files.len() == 1
            && wj_files[0].file_name().and_then(|n| n.to_str()) == Some("mod.wj");
        if wj_files.len() > 1 || is_root_mod_wj {
            // Pass the directory, not the file
            let source_dir = if path.is_file() {
                path.parent().unwrap_or(path)
            } else {
                path
            };
            build_utils::generate_nested_module_structure(source_dir, output)?;

            // CLEANUP: Remove stale .rs files that conflict with generated directory modules
            // Example: If lighting/ directory exists with mod.rs, remove lighting.rs (stale FFI file)
            build_utils::cleanup_stale_module_files(output)?;
        }

        // THE WINDJAMMER WAY: Detect component projects AFTER processing ALL files.
        // A component project is one where ALL files have no stdlib/external imports AND
        // a Cargo.toml already exists (it was generated by another mechanism).
        // Previously this check was inside the per-file loop, which meant the FIRST file
        // with empty imports would set the flag, preventing Cargo.toml regeneration for
        // the entire project - even if other files had imports. This caused stale Cargo.toml
        // files from previous builds to persist with incorrect targets.
        let is_component_project = all_stdlib_modules.is_empty()
            && all_external_crates.is_empty()
            && output.join("Cargo.toml").exists()
            && !output.join("lib.rs").exists(); // If lib.rs exists, always regenerate

        // Create Cargo.toml with stdlib and external dependencies (unless it's a component project)
        if !is_component_project {
            // THE WINDJAMMER WAY: Filter out internal modules from external_crates
            // Any module that has a .wj file in the project should NOT be an external dependency
            // CRITICAL FIX: Discover ALL .wj files recursively, not just the root file!
            let source_dir = if path.is_file() {
                path.parent().unwrap_or(path)
            } else {
                path
            };
            let all_wj_files_in_project = find_wj_files(source_dir).unwrap_or_default();
            let internal_modules: HashSet<String> = all_wj_files_in_project
                .iter()
                .filter_map(|f| {
                    f.file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .collect();

            let filtered_external_crates: Vec<String> = all_external_crates
                .into_iter()
                .filter(|crate_name| {
                    // Check both with hyphens and underscores (Cargo uses hyphens, files use underscores)
                    let crate_name_normalized = crate_name.replace('-', "_");
                    let is_internal = internal_modules.contains(crate_name)
                        || internal_modules.contains(&crate_name_normalized);
                    // Keep only crates that are NOT internal modules
                    !is_internal
                })
                .collect();

            // THE WINDJAMMER WAY: Scan generated Rust code for external crate usage
            // This catches dependencies like windjammer_app:: that are passed through
            // without being marked as external during compilation
            let mut rust_code_deps = HashSet::new();
            if let Ok(entries) = std::fs::read_dir(output) {
                for entry in entries.flatten() {
                    if let Some(filename) = entry.file_name().to_str() {
                        if filename.ends_with(".rs") {
                            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                                rust_code_deps
                                    .extend(RustBackend::extract_external_dependencies(&content));
                            }
                        }
                    }
                }
            }

            // Merge filtered_external_crates with rust_code_deps
            let builtin_skip = ["crate", "super", "windjammer", "windjammer_runtime"];
            let mut combined_external_crates = filtered_external_crates;
            for dep in rust_code_deps {
                if !combined_external_crates.contains(&dep) && !builtin_skip.contains(&dep.as_str())
                {
                    combined_external_crates.push(dep);
                }
            }

            cargo_integration::create_cargo_toml_with_deps(
                output,
                &all_stdlib_modules,
                &combined_external_crates,
                target,
                source_dir,
            )?;
        }

        println!("\n{} Transpilation complete!", "Success!".green().bold());
        println!("Output directory: {:?}", output);
        println!("\nTo run the generated code:");
        println!("  cd {:?}", output);
        println!("  cargo run");
        println!("\nOr use 'windjammer check' to see any Rust compilation errors");
        Ok(())
    } else {
        println!("\n{} Compilation failed with errors", "Error:".red().bold());
        anyhow::bail!("Compilation failed")
    }
}

#[allow(dead_code)]
pub fn find_wj_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("wj") {
            // THE WINDJAMMER WAY: If the file is mod.wj, find ALL .wj files in the parent directory
            // This is because mod.wj implies a multi-file project structure
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
        // Recursively find all .wj files in subdirectories
        find_wj_files_recursive(path, &mut files)?;
    }

    files.sort();
    Ok(files)
}

/// Recursively find all .wj files in a directory tree
fn find_wj_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("wj") {
            files.push(path);
        } else if path.is_dir() {
            find_wj_files_recursive(&path, files)?;
        }
    }
    Ok(())
}
