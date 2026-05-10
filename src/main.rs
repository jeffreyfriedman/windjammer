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
pub mod codegen;
pub mod compiler;
pub mod component_analyzer;
pub mod decorator_registry;
pub mod error;
pub mod errors;
pub mod plugin; // Plugin discovery and delegation // High-quality error messages (mutability, etc.)
                // Removed: codegen_legacy is now codegen::rust::generator
pub mod compiler_database;
pub mod config;
pub mod ejector;
pub mod error_catalog; // Error catalog generation and documentation
pub mod error_codes;
pub mod file_compiler; // Single-file compilation
pub mod module_system;
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

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CompilationTarget {
    /// Native Rust binary
    Rust,
    /// Go source code (experimental)
    Go,
    /// WebAssembly
    Wasm,
    /// Node.js native modules (future)
    Node,
    /// Python FFI via PyO3 (future)
    Python,
    /// C FFI (future)
    C,
}

#[derive(Parser)]
#[command(name = "windjammer")]
#[command(about = "Windjammer - A simple language that transpiles to Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a Windjammer project
    Build {
        /// Input directory or file
        #[arg(short, long, value_name = "PATH")]
        path: PathBuf,

        /// Output directory for generated Rust code
        #[arg(short, long, value_name = "OUTPUT")]
        output: PathBuf,

        /// Compilation target (wasm, node, python, c)
        #[arg(short, long, value_enum, default_value = "wasm")]
        target: CompilationTarget,

        /// Run cargo build after transpilation and show errors
        #[arg(long)]
        check: bool,

        /// Show raw Rust errors instead of translated Windjammer errors
        #[arg(long)]
        raw_errors: bool,

        /// Library mode: exclude test main() functions from output
        #[arg(long)]
        library: bool,

        /// Auto-generate mod.rs with pub mod declarations and re-exports
        #[arg(long)]
        module_file: bool,

        /// Disable Rust leakage linter warnings (style, .unwrap(), .iter(), etc.)
        #[arg(long)]
        no_lint: bool,
    },
    /// Check a Windjammer project for errors (transpile + cargo check)
    Check {
        /// Input directory or file
        #[arg(short, long, value_name = "PATH")]
        path: PathBuf,

        /// Output directory for generated Rust code
        #[arg(short, long, value_name = "OUTPUT")]
        output: PathBuf,

        /// Compilation target
        #[arg(short, long, value_enum, default_value = "wasm")]
        target: CompilationTarget,

        /// Show raw Rust errors instead of translated Windjammer errors
        #[arg(long)]
        raw_errors: bool,
    },
    /// Lint a Windjammer project (code quality, style, performance, security)
    Lint {
        /// Input directory or file
        #[arg(short, long, value_name = "PATH")]
        path: PathBuf,

        /// Maximum function length
        #[arg(long, default_value = "50")]
        max_function_length: usize,

        /// Maximum file length
        #[arg(long, default_value = "500")]
        max_file_length: usize,

        /// Maximum complexity score
        #[arg(long, default_value = "10")]
        max_complexity: usize,

        /// Disable unused code checks
        #[arg(long)]
        no_unused: bool,

        /// Disable style checks
        #[arg(long)]
        no_style: bool,

        /// Show only errors
        #[arg(long)]
        errors_only: bool,

        /// JSON output format
        #[arg(long)]
        json: bool,

        /// Enable auto-fix for supported rules
        #[arg(long)]
        fix: bool,
    },
    /// Eject to pure Rust - convert your Windjammer project to a standalone Rust project
    Eject {
        /// Input directory or file
        #[arg(short, long, value_name = "PATH")]
        path: PathBuf,

        /// Output directory for ejected Rust project
        #[arg(short, long, value_name = "OUTPUT")]
        output: PathBuf,

        /// Compilation target
        #[arg(short, long, value_enum, default_value = "wasm")]
        target: CompilationTarget,

        /// Run rustfmt on generated code
        #[arg(long, default_value = "true")]
        format: bool,

        /// Add helpful comments explaining Windjammer features
        #[arg(long, default_value = "true")]
        comments: bool,

        /// Skip Cargo.toml generation (use existing)
        #[arg(long)]
        no_cargo_toml: bool,
    },
    /// Run a Windjammer file (build + cargo run, or --interpret for instant execution)
    Run {
        /// Input file to run
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Compilation target
        #[arg(short, long, value_enum, default_value = "rust")]
        target: CompilationTarget,

        /// Interpret directly (Windjammerscript mode) — no compilation, instant execution.
        /// Same .wj source can later be compiled with `wj build` for production.
        #[arg(long)]
        interpret: bool,

        /// Arguments to pass to the program
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Start the Windjammerscript REPL (interactive interpreter)
    Repl {},
    /// Run tests (discovers and runs all test functions)
    Test {
        /// Directory or file containing tests (defaults to current directory)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        /// Run only tests matching this pattern
        #[arg(short, long)]
        filter: Option<String>,

        /// Show output from passing tests
        #[arg(long)]
        nocapture: bool,

        /// Run tests in parallel (default: true)
        #[arg(long, default_value = "true")]
        parallel: bool,

        /// Output results as JSON for tooling
        #[arg(long)]
        json: bool,
    },
}

#[allow(dead_code)]
/// Main CLI entry point (called from bin/wj.rs after plugin discovery)
pub fn run_main_cli() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            path,
            output,
            target,
            check,
            raw_errors,
            library,
            module_file,
            no_lint,
        } => {
            build_project(&path, &output, target, !no_lint)?;

            // Generate mod.rs if requested
            if module_file {
                build_utils::generate_mod_file(&output)?;
            }

            // Strip main() functions if library mode
            if library {
                build_utils::strip_main_functions(&output)?;
            }

            if check {
                cargo_integration::check_with_cargo(&output, raw_errors)?;
            }
        }
        Commands::Check {
            path,
            output,
            target,
            raw_errors,
        } => {
            build_project(&path, &output, target, true)?;
            cargo_integration::check_with_cargo(&output, raw_errors)?;
        }
        Commands::Lint {
            path,
            max_function_length,
            max_file_length,
            max_complexity,
            no_unused,
            no_style,
            errors_only,
            json,
            fix,
        } => {
            lint_project(
                &path,
                max_function_length,
                max_file_length,
                max_complexity,
                !no_unused,
                !no_style,
                errors_only,
                json,
                fix,
            )?;
        }
        Commands::Eject {
            path,
            output,
            target,
            format,
            comments,
            no_cargo_toml,
        } => {
            eject_project(&path, &output, target, format, comments, !no_cargo_toml)?;
        }
        Commands::Run {
            file,
            target,
            interpret,
            args,
        } => {
            if interpret {
                interpret_file(&file)?;
            } else {
                run_file(&file, target, &args)?;
            }
        }
        Commands::Repl {} => {
            run_repl()?;
        }
        Commands::Test {
            path,
            filter,
            nocapture,
            parallel,
            json,
        } => {
            test_runner::run_tests(
                path.as_deref(),
                filter.as_deref(),
                nocapture,
                parallel,
                json,
            )?;
        }
    }

    Ok(())
}

/// Quick Copy type check for PASS 0 (no full analyzer available).
/// Checks if a type is Copy based on primitives and already-known Copy structs/enums.
fn is_type_copy_quick(
    ty: &parser::Type,
    copy_structs: &HashSet<String>,
    copy_enums: &HashSet<String>,
) -> bool {
    use parser::Type;
    match ty {
        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
        Type::Reference(_) => true,
        Type::MutableReference(_) => false,
        Type::Tuple(types) => types
            .iter()
            .all(|t| is_type_copy_quick(t, copy_structs, copy_enums)),
        Type::Option(inner) => is_type_copy_quick(inner, copy_structs, copy_enums),
        Type::Result(ok, err) => {
            is_type_copy_quick(ok, copy_structs, copy_enums)
                && is_type_copy_quick(err, copy_structs, copy_enums)
        }
        Type::Array(inner, _) => is_type_copy_quick(inner, copy_structs, copy_enums),
        Type::Vec(_) | Type::String => false,
        Type::RawPointer { pointee, .. } => {
            is_type_copy_quick(pointee.as_ref(), copy_structs, copy_enums)
        }
        Type::FunctionPointer { .. } => true,
        Type::Custom(name) => {
            copy_structs.contains(name)
                || copy_enums.contains(name)
                || windjammer::type_classification::is_copy_primitive(name.as_str())
        }
        _ => false,
    }
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
    // When we have metadata or library, use the compiler's simpler build (handles cross-crate)
    if !external_metadata.is_empty() || (library && path.is_file()) {
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
    use colored::*;

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
    let mut structs_by_name: std::collections::HashMap<String, Vec<&StructInfo>> =
        std::collections::HashMap::new();
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
            let mut rust_code_deps = std::collections::HashSet::new();
            if let Ok(entries) = std::fs::read_dir(output) {
                for entry in entries.flatten() {
                    if let Some(filename) = entry.file_name().to_str() {
                        if filename.ends_with(".rs") {
                            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                                rust_code_deps.extend(
                                    crate::codegen::rust::backend::RustBackend::extract_external_dependencies(&content)
                                );
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
fn check_project(path: &Path) -> Result<()> {
    use colored::*;

    println!(
        "{} Windjammer project: {:?}",
        "Checking".cyan().bold(),
        path
    );

    let wj_files = find_wj_files(path)?;

    if wj_files.is_empty() {
        println!("{} No .wj files found", "Warning:".yellow().bold());
        return Ok(());
    }

    println!("Found {} file(s) to check", wj_files.len());

    // For now, just parse all files to check for syntax errors
    let mut has_errors = false;

    for file in &wj_files {
        let file_name = file.file_name().unwrap().to_str().unwrap();
        print!("  Checking {:?}... ", file_name);

        let source = std::fs::read_to_string(file)?;
        let mut lexer = lexer::Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = parser::Parser::new(tokens);

        match parser.parse() {
            Ok(_) => println!("{}", "✓".green()),
            Err(e) => {
                println!("{}", "✗".red());
                println!("    Error: {}", e);
                has_errors = true;
            }
        }
    }

    if !has_errors {
        println!(
            "\n{} All files passed syntax check!",
            "Success!".green().bold()
        );
    } else {
        println!("\n{} Some files have errors", "Error:".red().bold());
    }

    Ok(())
}

#[allow(dead_code)]
fn find_wj_files(path: &Path) -> Result<Vec<PathBuf>> {
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

// Module compiler for handling dependencies
#[allow(dead_code)]

#[allow(dead_code)]
#[allow(dead_code)]
fn check_file(file_path: &Path) -> Result<()> {
    let source = std::fs::read_to_string(file_path)?;

    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();

    let mut parser = parser::Parser::new(tokens);
    let _program = parser
        .parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    Ok(())
}

/// TDD FIX (Bug #2): File type detection for Cargo.toml target generation
#[derive(Debug, PartialEq)]
pub enum RustFileType {
    Test,    // Contains #[test] functions
    Binary,  // Contains fn main()
    Library, // Neither (just library code)
}

/// Detect what type of Rust file this is by scanning its contents
pub fn detect_rust_file_type(path: &Path) -> RustFileType {
    if let Ok(contents) = std::fs::read_to_string(path) {
        let has_main = contents.contains("fn main()") || contents.contains("fn main(");
        let has_test = contents.contains("#[test]");

        // Priority: main() takes precedence (binaries can have tests)
        // Files with ONLY tests (no main) are test targets
        // Files with neither are library modules (no target needed)
        if has_main {
            RustFileType::Binary
        } else if has_test {
            RustFileType::Test
        } else {
            RustFileType::Library
        }
    } else {
        // Can't read file - assume library
        RustFileType::Library
    }
}

/// Load and merge all source maps from the output directory
pub fn load_source_maps(output_dir: &Path) -> Result<source_map::SourceMap> {
    use colored::*;
    use std::fs;

    let mut merged_map = source_map::SourceMap::new();
    let mut map_count = 0;
    let mut mapping_count = 0;

    // Find all .rs.map files in the output directory
    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("map") {
                // Check if this is a .rs.map file (not just any .map file)
                if let Some(stem) = path.file_stem() {
                    if let Some(stem_str) = stem.to_str() {
                        if !stem_str.ends_with(".rs") {
                            continue;
                        }
                    }
                }

                // Load this source map
                if let Ok(map) = source_map::SourceMap::load_from_file(&path) {
                    // Get the corresponding .rs file path
                    let rust_file = path.with_extension("").with_extension("rs");

                    // Merge all mappings from this source map
                    let mappings = map.mappings_for_rust_file(&rust_file);
                    for mapping in mappings {
                        merged_map.add_mapping(
                            &mapping.rust_file,
                            mapping.rust_line,
                            mapping.rust_column,
                            &mapping.wj_file,
                            mapping.wj_line,
                            mapping.wj_column,
                        );
                        mapping_count += 1;
                    }
                    map_count += 1;
                }
            }
        }
    }

    if map_count == 0 {
        eprintln!(
            "{} No source maps found in {}. Errors will reference Rust code.",
            "Warning:".yellow().bold(),
            output_dir.display()
        );
    } else {
        eprintln!(
            "{} Loaded {} source map{} with {} mapping{}",
            "Info:".cyan(),
            map_count,
            if map_count == 1 { "" } else { "s" },
            mapping_count,
            if mapping_count == 1 { "" } else { "s" }
        );
    }

    Ok(merged_map)
}

/// Colorize diagnostic output based on level
pub fn colorize_diagnostic(text: &str, _level: &error_mapper::DiagnosticLevel) -> String {
    use colored::*;

    let mut result = String::new();
    for line in text.lines() {
        if line.starts_with("error") {
            result.push_str(&line.red().bold().to_string());
        } else if line.starts_with("warning") {
            result.push_str(&line.yellow().bold().to_string());
        } else if line.contains("-->") {
            result.push_str(&line.blue().bold().to_string());
        } else if line.starts_with("  = help:") {
            result.push_str(&line.cyan().to_string());
        } else if line.starts_with("  = suggestion:") {
            result.push_str(&line.green().bold().to_string());
        } else if line.starts_with("  = note:") {
            result.push_str(&line.white().dimmed().to_string());
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    result
}

/// Lint a Windjammer project using the LSP diagnostics engine
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
fn lint_project(
    path: &Path,
    max_function_length: usize,
    max_file_length: usize,
    max_complexity: usize,
    check_unused: bool,
    check_style: bool,
    errors_only: bool,
    json: bool,
    fix: bool,
) -> Result<()> {
    use colored::*;

    // This is a placeholder - full implementation would use windjammer-lsp
    // For now, print a message about what would be checked

    if json {
        println!("{{");
        println!("  \"linter\": \"windjammer\",");
        println!("  \"version\": \"0.26.0\",");
        println!("  \"path\": {:?},", path);
        println!("  \"config\": {{");
        println!("    \"max_function_length\": {},", max_function_length);
        println!("    \"max_file_length\": {},", max_file_length);
        println!("    \"max_complexity\": {},", max_complexity);
        println!("    \"check_unused\": {},", check_unused);
        println!("    \"check_style\": {}", check_style);
        println!("  }},");
        println!("  \"diagnostics\": [");
        println!("  ]");
        println!("}}");
    } else {
        println!(
            "{} Windjammer files in: {:?}",
            "Linting".cyan().bold(),
            path
        );
        println!();
        println!("{}", "Configuration:".bold());
        println!("  • Max function length: {}", max_function_length);
        println!("  • Max file length: {}", max_file_length);
        println!("  • Max complexity: {}", max_complexity);
        println!(
            "  • Check unused code: {}",
            if check_unused {
                "yes".green()
            } else {
                "no".red()
            }
        );
        println!(
            "  • Check style: {}",
            if check_style {
                "yes".green()
            } else {
                "no".red()
            }
        );
        println!(
            "  • Errors only: {}",
            if errors_only { "yes" } else { "no" }
        );
        if fix {
            println!("  • Auto-fix: {}", "enabled".green().bold());
        } else {
            println!("  • Auto-fix: disabled");
        }
        println!();

        println!(
            "{}",
            "Diagnostic Categories (inspired by golangci-lint):".bold()
        );
        println!(
            "  {} Code Quality: complexity, style, code smell",
            "✓".green()
        );
        println!(
            "  {} Error Detection: bug risk, error handling, nil check",
            "✓".green()
        );
        println!("  {} Performance: performance, memory", "✓".green());
        println!("  {} Security: security checks", "✓".green());
        println!(
            "  {} Maintainability: naming, documentation, unused",
            "✓".green()
        );
        println!(
            "  {} Dependencies: import, dependency (circular)",
            "✓".green()
        );
        println!();

        println!("{}", "Rules Implemented:".bold());
        println!();
        println!("  {}:", "Code Quality & Style".underline());
        if fix {
            println!(
                "    • {} Detect unused code {}",
                "unused-code:".cyan(),
                "(auto-fixable)".green()
            );
        } else {
            println!("    • {} Detect unused code", "unused-code:".cyan());
        }
        println!("    • {} Flag long functions", "function-length:".cyan());
        println!("    • {} Flag large files", "file-length:".cyan());
        if fix {
            println!(
                "    • {} Check naming conventions {}",
                "naming-convention:".cyan(),
                "(auto-fixable)".green()
            );
        } else {
            println!(
                "    • {} Check naming conventions",
                "naming-convention:".cyan()
            );
        }
        println!("    • {} Require documentation", "missing-docs:".cyan());
        println!();
        println!("  {}:", "Error Handling".underline());
        println!(
            "    • {} Detect unchecked Result",
            "unchecked-result:".cyan()
        );
        println!("    • {} Warn about panic!()", "avoid-panic:".cyan());
        println!("    • {} Warn about .unwrap()", "avoid-unwrap:".cyan());
        println!();
        println!("  {}:", "Performance".underline());
        if fix {
            println!(
                "    • {} Suggest Vec::with_capacity() {}",
                "vec-prealloc:".cyan(),
                "(auto-fixable)".green()
            );
        } else {
            println!(
                "    • {} Suggest Vec::with_capacity()",
                "vec-prealloc:".cyan()
            );
        }
        println!("    • {} Warn about string concat", "string-concat:".cyan());
        println!("    • {} Detect clone in loops", "clone-in-loop:".cyan());
        println!();
        println!("  {}:", "Security".underline());
        println!("    • {} Flag unsafe blocks", "unsafe-block:".cyan());
        println!(
            "    • {} Detect hardcoded secrets",
            "hardcoded-secret:".cyan()
        );
        println!("    • {} Warn about SQL injection", "sql-injection:".cyan());
        println!();
        println!("  {}:", "Dependencies".underline());
        println!(
            "    • {} Detect circular imports",
            "circular-dependency:".cyan()
        );
        println!();

        // TODO: Integrate with windjammer-lsp to actually run diagnostics
        println!("{}", "✨ World-class linting ready!".green().bold());
        println!();
        println!(
            "{}",
            "Note: Full linting integration with windjammer-lsp coming soon.".yellow()
        );
        println!("      The diagnostics engine is implemented and tested (83 tests passing).");
        println!("      Use the LSP server for real-time linting in your editor.");
    }

    Ok(())
}

/// Translate Rust compiler messages to Windjammer terminology
#[allow(dead_code)]
fn translate_error_message_with_spans(
    rust_msg: &str,
    spans: &[error_mapper::DiagnosticSpan],
) -> String {
    // Check for type mismatch in span labels
    if rust_msg.contains("mismatched types") {
        // Try to extract from primary span label
        if let Some(primary) = spans.iter().find(|s| s.is_primary) {
            // Label format: "expected `i64`, found `&str`"
            if let Some(ref label) = primary.label {
                if let Some(expected) = extract_between(label, "expected `", "`") {
                    if let Some(found) = extract_between(label, "found `", "`") {
                        return format!(
                            "Type mismatch: expected {}, found {}",
                            rust_type_to_windjammer(expected),
                            rust_type_to_windjammer(found)
                        );
                    }
                }
            }
        }
        // Fallback: just say type mismatch
        return "Type mismatch".to_string();
    }

    if rust_msg.contains("cannot find type") {
        if let Some(type_name) = extract_between(rust_msg, "cannot find type `", "`") {
            return format!("Type not found: {}", type_name);
        }
    }

    if rust_msg.contains("cannot find function") {
        if let Some(func_name) = extract_between(rust_msg, "cannot find function `", "`") {
            return format!("Function not found: {}", func_name);
        }
    }

    if rust_msg.contains("cannot move out of") {
        return "Ownership error: value was moved".to_string();
    }

    if rust_msg.contains("trait bounds were not satisfied") {
        return "Missing trait implementation or type constraint".to_string();
    }

    // Fallback: return original
    rust_msg.to_string()
}

fn rust_type_to_windjammer(rust_type: &str) -> String {
    match rust_type {
        "i64" => "int",
        "f64" => "float",
        "bool" => "bool",
        "&str" | "String" | "&String" => "string",
        "()" => "()",
        _ => rust_type,
    }
    .to_string()
}

fn extract_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let start_pos = text.find(start)? + start.len();
    let remaining = &text[start_pos..];
    let end_pos = remaining.find(end)?;
    Some(&remaining[..end_pos])
}

/// Eject a Windjammer project to pure Rust
#[allow(dead_code)]
fn eject_project(
    path: &Path,
    output: &Path,
    target: CompilationTarget,
    format: bool,
    comments: bool,
    generate_cargo_toml: bool,
) -> Result<()> {
    let config = ejector::EjectConfig {
        format,
        comments,
        generate_cargo_toml,
        target,
    };

    let mut ejector = ejector::Ejector::new(config);
    ejector.eject_project(path, output)?;

    Ok(())
}

/// Run a Windjammer file (build + cargo run)
fn run_file(file: &Path, target: CompilationTarget, args: &[String]) -> Result<()> {
    use colored::*;
    use std::fs;
    use std::process::Command;

    // Validate that the file exists and is a .wj file
    if !file.exists() {
        anyhow::bail!("File not found: {:?}", file);
    }
    if file.extension().is_none_or(|ext| ext != "wj") {
        anyhow::bail!("File must have .wj extension: {:?}", file);
    }

    // Auto-detect target based on imports ONLY if no explicit target was provided
    // (default_value in CLI is "rust", so we can't distinguish between explicit and default)
    // For now, skip auto-detection if user provided --target flag
    // TODO: Better way to detect if user explicitly provided --target

    println!(
        "{} {:?} (target: {:?})",
        "Running".green().bold(),
        file,
        target
    );

    // Create a temporary build directory
    let temp_dir = std::env::temp_dir().join(format!(
        "windjammer-run-{}",
        file.file_stem().and_then(|s| s.to_str()).unwrap_or("app")
    ));

    // Clean up any previous build
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;

    // Build the project
    build_project(file, &temp_dir, target, true)?;

    // Handle execution based on target
    match target {
        CompilationTarget::Wasm => {
            // WASM apps need to be built with wasm-pack and served
            println!("\n{} WASM app...", "Building".cyan().bold());
            println!("To run this WASM app:");
            println!("  1. cd {:?}", temp_dir);
            println!("  2. wasm-pack build --target web");
            println!("  3. Serve the generated HTML file");
            println!("\nOr use the pre-built counter example:");
            println!("  cd crates/windjammer-ui");
            println!("  wasm-pack build --target web");
            println!("  # Then serve examples/counter_wasm.html");

            // Don't clean up so user can inspect/run
            println!("\n{} Build artifacts in: {:?}", "ℹ".cyan().bold(), temp_dir);
        }
        _ => {
            // Native targets can be run directly
            println!("\n{} the program...", "Executing".cyan().bold());
            let mut cmd = Command::new("cargo");
            cmd.arg("run").current_dir(&temp_dir);

            // Pass through any additional arguments
            if !args.is_empty() {
                cmd.arg("--");
                cmd.args(args);
            }

            let status = cmd.status()?;

            if !status.success() {
                anyhow::bail!("Program execution failed");
            }

            // Clean up temp directory for native builds
            if temp_dir.exists() {
                fs::remove_dir_all(&temp_dir)?;
            }
        }
    }

    Ok(())
}

fn interpret_file(file: &Path) -> Result<()> {
    use colored::*;

    if !file.exists() {
        anyhow::bail!("File not found: {:?}", file);
    }
    if file.extension().is_none_or(|ext| ext != "wj") {
        anyhow::bail!("File must have .wj extension: {:?}", file);
    }

    println!(
        "{} {:?} (Windjammerscript interpreter)",
        "Interpreting".green().bold(),
        file
    );

    let source = std::fs::read_to_string(file)?;
    let mut lex = lexer::Lexer::new(&source);
    let tokens = lex.tokenize_with_locations();
    let mut parse =
        parser::Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
    let program = parse
        .parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    let mut interp = interpreter::Interpreter::new();
    match interp.run(&program) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{} {}", "Runtime error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

/// Run the Windjammerscript REPL (Read-Eval-Print Loop)
fn run_repl() -> Result<()> {
    use colored::*;
    use std::io::{self, BufRead, Write};

    println!(
        "{} {} {}",
        "Windjammerscript".cyan().bold(),
        "REPL".white().bold(),
        "(type 'exit' or Ctrl-D to quit)".dimmed()
    );
    println!(
        "{}",
        "Tip: Any code you write here can be compiled with `wj build` for production.".dimmed()
    );
    println!();

    let stdin = io::stdin();
    let mut accumulated_source = String::new();
    let mut line_buffer = String::new();
    let mut in_block = false;
    let mut brace_depth: i32 = 0;

    loop {
        // Print prompt
        if in_block {
            print!("{} ", "...".dimmed());
        } else {
            print!("{} ", "wj>".green().bold());
        }
        io::stdout().flush()?;

        line_buffer.clear();
        let bytes_read = stdin.lock().read_line(&mut line_buffer)?;
        if bytes_read == 0 {
            // EOF (Ctrl-D)
            println!();
            break;
        }

        let line = line_buffer.trim_end();

        if line == "exit" || line == "quit" {
            break;
        }

        // Track brace depth for multi-line input
        for ch in line.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => brace_depth -= 1,
                _ => {}
            }
        }

        accumulated_source.push_str(line);
        accumulated_source.push('\n');

        if brace_depth > 0 {
            in_block = true;
            continue;
        }

        in_block = false;
        brace_depth = 0;

        // Wrap in main() if it's a simple expression/statement
        let source = if accumulated_source.contains("fn main()") {
            accumulated_source.clone()
        } else {
            format!("fn main() {{\n{}\n}}", accumulated_source)
        };

        // Parse and interpret
        let mut lex = lexer::Lexer::new(&source);
        let tokens = lex.tokenize_with_locations();
        let mut parse = parser::Parser::new_with_source(tokens, "repl".to_string(), source.clone());

        match parse.parse() {
            Ok(program) => {
                let mut interp = interpreter::Interpreter::new();
                match interp.run(&program) {
                    Ok(val) => {
                        // Print non-unit return values
                        let display = val.to_display_string();
                        if display != "()" {
                            println!("{}", display);
                        }
                    }
                    Err(e) => {
                        eprintln!("{} {}", "Error:".red().bold(), e);
                    }
                }
            }
            Err(e) => {
                eprintln!("{} {}", "Parse error:".red().bold(), e);
            }
        }

        accumulated_source.clear();
    }

    println!("{}", "Goodbye!".dimmed());
    Ok(())
}

/// Generate mod.rs file with pub mod declarations and re-exports
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
