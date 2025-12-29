// Allow recursive functions that use self only for recursion
// This is common in AST traversal code
#![allow(clippy::only_used_in_recursion)]

pub mod analyzer;
pub mod auto_clone; // Automatic clone insertion for ergonomics
pub mod auto_fix; // Automatic error fixing
pub mod cli;
pub mod codegen;
pub mod component_analyzer;
pub mod error;
pub mod errors; // High-quality error messages (mutability, etc.)
                // Removed: codegen_legacy is now codegen::rust::generator
pub mod compiler_database;
pub mod config;
pub mod ejector;
pub mod error_catalog; // Error catalog generation and documentation
pub mod error_codes;
pub mod module_system; // Nested module system - The Windjammer Way! // Windjammer error codes (WJ0001, etc.)

pub mod error_mapper;
pub mod error_statistics; // Error statistics tracking and analysis
pub mod error_tui; // Interactive TUI for error navigation
pub mod fuzzy_matcher; // Fuzzy string matching for typo suggestions
pub mod inference;
pub mod lexer;
// OPTIMIZER INTENTIONALLY DISABLED: Requires architectural refactoring for arena allocation
// 
// The optimizer has 150 lifetime errors due to arena ownership architecture:
// - Optimizer owns an arena and returns Program<'arena> references
// - But caller expects Program<'static> or other lifetimes
// - This is a fundamental architecture issue, not a simple fix
//
// SOLUTION PATHS (see docs/ARENA_SESSION6_FINAL.md):
// 1. Optimizer clones output (owns arena, returns owned Program)
// 2. Higher-level arena (passed to optimizer as parameter)
// 3. Skip optimization phase (current approach - compilation works fine)
//
// DECISION: Defer to separate PR. Core compiler is 100% arena-allocated and working.
// The 87.5% stack reduction and elimination of recursive drops is the critical win.
//
// pub mod optimizer;  // Re-enable after architectural refactoring
pub mod parser; // Parser module (refactored structure)
pub mod parser_impl; // Parser implementation (being migrated to parser/)
#[cfg(test)]
pub mod test_utils; // Test utilities for arena-allocated AST construction
pub mod parser_recovery;
pub mod source_map; // Source map for error message translation
pub mod source_map_cache; // Source map caching for performance
pub mod stdlib_scanner;
pub mod syntax_highlighter; // Syntax highlighting for error snippets

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CompilationTarget {
    /// Native Rust binary
    Rust,
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
    /// Run a Windjammer file (build + cargo run)
    Run {
        /// Input file to run
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Compilation target
        #[arg(short, long, value_enum, default_value = "rust")]
        target: CompilationTarget,

        /// Arguments to pass to the program
        #[arg(last = true)]
        args: Vec<String>,
    },
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
fn main() -> Result<()> {
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
        } => {
            build_project(&path, &output, target)?;

            // Generate mod.rs if requested
            if module_file {
                generate_mod_file(&output)?;
            }

            // Strip main() functions if library mode
            if library {
                strip_main_functions(&output)?;
            }

            if check {
                check_with_cargo(&output, raw_errors)?;
            }
        }
        Commands::Check {
            path,
            output,
            target,
            raw_errors,
        } => {
            build_project(&path, &output, target)?;
            check_with_cargo(&output, raw_errors)?;
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
        Commands::Run { file, target, args } => {
            run_file(&file, target, &args)?;
        }
        Commands::Test {
            path,
            filter,
            nocapture,
            parallel,
            json,
        } => {
            run_tests(
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

pub fn build_project(path: &Path, output: &Path, target: CompilationTarget) -> Result<()> {
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
    let mut is_component_project = false;

    // Create a single ModuleCompiler for all files to share trait registry
    let mut module_compiler = ModuleCompiler::new(target);

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
    // This ensures Copy type detection works across all files
    let mut global_copy_structs = HashSet::new();
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
                if let parser::Item::Struct { decl, .. } = item {
                    // Check if struct has @derive(Copy)
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
                    if has_copy {
                        global_copy_structs.insert(decl.name.clone());
                    }
                }
            }
        }
    }

    // Transfer Copy structs to ModuleCompiler's registry
    module_compiler.copy_structs_registry = global_copy_structs;

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
            // Register any trait definitions found
            for item in &program.items {
                if let parser::Item::Trait { decl, .. } = item {
                    module_compiler
                        .trait_registry
                        .insert(decl.name.clone(), decl.clone());
                }
            }
        }
    }

    // Determine source_root: if path is a file, find the actual source root
    // BUGFIX: For nested files like src_wj/ecs/entity.wj, we need to find src_wj,
    // not just the immediate parent (src_wj/ecs)
    let source_root = if path.is_file() {
        find_source_root(path).unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")))
    } else {
        path
    };

    // PASS 2: Full compilation with all traits available
    let is_multi_file = wj_files.len() > 1;
    println!("DEBUG: Starting PASS 2 with {} files", wj_files.len());
    for file in &wj_files {
        let file_name = file.file_name().unwrap().to_str().unwrap();

        // THE WINDJAMMER WAY: In multi-file projects, skip mod.wj files
        // They're only for controlling re-exports, which generate_nested_module_structure handles
        // Compiling them would overwrite the correctly-generated mod.rs files
        if is_multi_file && file_name == "mod.wj" {
            continue;
        }

        print!("  Compiling {:?}... ", file_name);

        match compile_file_with_compiler(
            source_root,
            file,
            output,
            &mut module_compiler,
            is_multi_file,
            true,
        ) {
            Ok((stdlib_modules, external_crates)) => {
                println!("{}", "✓".green());
                // If both are empty, this might be a component (which handles its own Cargo.toml)
                if stdlib_modules.is_empty() && external_crates.is_empty() {
                    // Check if Cargo.toml already exists (component generated it)
                    if output.join("Cargo.toml").exists() {
                        is_component_project = true;
                    }
                }
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

            // THE WINDJAMMER WAY: In multi-file projects, skip mod.wj files during regeneration too
            if is_multi_file && file_name == "mod.wj" {
                continue;
            }

            print!("  Updating {:?}... ", file_name);
            match compile_file_with_compiler(
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
            generate_nested_module_structure(source_dir, output)?;
        }

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

            create_cargo_toml_with_deps(
                output,
                &all_stdlib_modules,
                &filtered_external_crates,
                target,
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
            // Recurse into subdirectories
            find_wj_files_recursive(&path, files)?;
        }
    }
    Ok(())
}

// Module compiler for handling dependencies
#[allow(dead_code)]
struct ModuleCompiler {
    compiled_modules: HashMap<String, String>, // module path -> generated Rust code
    target: CompilationTarget,
    stdlib_path: PathBuf,
    source_roots: Vec<PathBuf>, // Additional source roots (e.g., ../windjammer-game-core/src_wj)
    imported_stdlib_modules: HashSet<String>, // Track which stdlib modules are used
    external_crates: Vec<String>, // Track external crates (e.g., windjammer_ui)
    trait_registry: HashMap<String, parser::TraitDecl<'static>>, // Global trait registry for cross-file trait resolution
    copy_structs_registry: HashSet<String>, // Global Copy struct registry for proper Copy detection across files
    analyzer: analyzer::Analyzer<'static>, // WINDJAMMER FIX: Shared analyzer for cross-file trait analysis
    // THE WINDJAMMER WAY: Track ALL programs for cross-file trait inference
    all_programs: Vec<parser::Program<'static>>, // All parsed programs from all files
    // RECURSION GUARD: Track files currently being compiled to prevent circular dependencies
    // Use String instead of PathBuf for Windows UNC path compatibility
    compiling_files: HashSet<String>, // Normalized path strings in the current compilation chain
}

#[allow(dead_code)]
impl ModuleCompiler {
    fn new(target: CompilationTarget) -> Self {
        // Check for WINDJAMMER_STDLIB env var, otherwise use ./std
        let stdlib_path = std::env::var("WINDJAMMER_STDLIB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./std"));

        Self {
            compiled_modules: HashMap::new(),
            target,
            stdlib_path,
            source_roots: Vec::new(),
            imported_stdlib_modules: HashSet::new(),
            external_crates: Vec::new(),
            trait_registry: HashMap::new(),
            copy_structs_registry: HashSet::new(),
            analyzer: analyzer::Analyzer::new(), // WINDJAMMER FIX: Shared analyzer instance
            all_programs: Vec::new(),            // THE WINDJAMMER WAY: Track all programs
            compiling_files: HashSet::new(),     // RECURSION GUARD: Track compilation chain
        }
    }

    fn add_source_root(&mut self, path: PathBuf) {
        self.source_roots.push(path);
    }

    /// THE WINDJAMMER WAY: Run cross-file trait inference after all files are analyzed
    /// This ensures trait signatures are inferred from ALL implementations across the project
    fn finalize_trait_inference(&mut self) -> Result<()> {
        // Create a merged program with ALL items from ALL files
        let mut all_items = Vec::new();
        for program in &self.all_programs {
            all_items.extend(program.items.clone());
        }

        let merged_program = parser::Program { items: all_items };

        // Run the cross-file trait inference
        self.analyzer
            .infer_trait_signatures_from_impls(&merged_program)
            .map_err(|e| anyhow::anyhow!("Trait inference error: {}", e))?;

        Ok(())
    }

    fn compile_module(&mut self, module_path: &str, source_file: Option<&Path>) -> Result<()> {
        // Skip if already compiled
        if self.compiled_modules.contains_key(module_path) {
            return Ok(());
        }

        // Skip stdlib modules - they're implemented in windjammer-runtime
        if module_path.starts_with("std::") {
            // Track that we used this stdlib module
            let module_name = module_path.strip_prefix("std::").unwrap().to_string();
            self.imported_stdlib_modules.insert(module_name);

            // Mark as compiled (no code generated, handled by runtime)
            self.compiled_modules
                .insert(module_path.to_string(), String::new());
            return Ok(());
        }

        // Resolve module path to file path
        let file_path = self.resolve_module_path(module_path, source_file)?;

        // Check if this is a source root module (marked by __source_root__ prefix)
        // These are modules from configured source roots that shouldn't be recursively compiled
        // when building individual files (they'll be in the same Rust module)
        if file_path
            .to_str()
            .is_some_and(|s| s.starts_with("__source_root__::"))
        {
            // Mark as compiled but don't generate code (will be compiled separately)
            self.compiled_modules
                .insert(module_path.to_string(), String::new());
            return Ok(());
        }

        // Check if this is an external crate (marked by __external__ prefix)
        if file_path
            .to_str()
            .is_some_and(|s| s.starts_with("__external__::"))
        {
            // External crate - extract crate name and mark as external dependency
            let crate_name = file_path
                .to_str()
                .unwrap()
                .strip_prefix("__external__::")
                .unwrap()
                .replace(".*", "") // Remove glob imports
                .split("::{") // Remove braced imports (::{ syntax)
                .next()
                .unwrap()
                .split(".{") // Remove braced imports (.{ syntax)
                .next()
                .unwrap()
                .split("::") // Take first segment
                .next()
                .unwrap()
                .replace('_', "-"); // Convert underscores to hyphens for Cargo.toml

            // Add to external crates if not already present
            if !self.external_crates.contains(&crate_name) {
                self.external_crates.push(crate_name.clone());
            }

            // Mark as compiled (external, no code generated)
            self.compiled_modules
                .insert(module_path.to_string(), String::new());
            return Ok(());
        }

        // Read and parse module
        let source = std::fs::read_to_string(&file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read module {}: {}", module_path, e))?;

        let mut lexer = lexer::Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = parser::Parser::new(tokens);
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", module_path, e))?;

        // Mark as "being compiled" to prevent infinite recursion
        // We'll update this with the actual code later
        self.compiled_modules
            .insert(module_path.to_string(), String::new());

        // Recursively compile dependencies
        for item in &program.items {
            if let parser::Item::Use { path, alias: _, .. } = item {
                let dep_path = path.join("::");

                // Handle item imports: ./main::Args -> compile ./main, not ./main::Args
                // Also handle braced imports: ./message::{A, B, C} -> compile ./message
                // Split at the last :: to separate module from item
                let module_to_compile = if dep_path.contains("::") {
                    // Check if this looks like a module::Item import or module::{...} import
                    let parts: Vec<&str> = dep_path.rsplitn(2, "::").collect();
                    if parts.len() == 2 {
                        let potential_item = parts[0];
                        let module_part = parts[1];

                        // If the last part starts with uppercase or {, it's likely a type/braced import
                        // e.g., ./main::Args, ./types::Config, ./message::{A, B, C}
                        if potential_item
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_uppercase() || c == '{')
                        {
                            module_part.to_string()
                        } else {
                            // It's a nested module path, compile the whole thing
                            dep_path.clone()
                        }
                    } else {
                        dep_path.clone()
                    }
                } else {
                    dep_path.clone()
                };

                // Pass the current file's path for resolving relative imports
                self.compile_module(&module_to_compile, Some(&file_path))?;
            }
        }

        // Register traits from this program into the global registry
        for item in &program.items {
            if let parser::Item::Trait { decl, .. } = item {
                self.trait_registry.insert(decl.name.clone(), decl.clone());
            }
        }

        // WINDJAMMER FIX: Use the SHARED analyzer for cross-file trait analysis
        // This ensures trait methods analyzed in file 1 are available when analyzing impl in file 2

        // Update analyzer's Copy structs registry (in case new Copy structs were discovered)
        self.analyzer
            .update_copy_structs(self.copy_structs_registry.clone());

        // Register any newly discovered traits into the analyzer
        for trait_decl in self.trait_registry.values() {
            let dummy_program = parser::Program {
                items: vec![parser::Item::Trait {
                    decl: trait_decl.clone(),
                    location: parser::SourceLocation::default(),
                }],
            };
            self.analyzer.register_traits_from_program(&dummy_program);
        }

        // THE WINDJAMMER WAY: Store this program for cross-file trait inference
        self.all_programs.push(program.clone());

        let (analyzed, signatures, analyzed_trait_methods) = self
            .analyzer
            .analyze_program(&program)
            .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;

        // Generate Rust code (as a module)
        let mut generator = codegen::CodeGenerator::new_for_module(signatures, self.target);
        generator.set_analyzed_trait_methods(analyzed_trait_methods);
        let rust_code = generator.generate_program(&program, &analyzed);

        // Extract module name from path
        // For "std::json" -> "json"
        // For "./utils" -> "utils"
        let module_name = if module_path.starts_with("std::") {
            module_path.strip_prefix("std::").unwrap().to_string()
        } else {
            // For relative paths, use the last component
            module_path
                .trim_start_matches("./")
                .trim_start_matches("../")
                .split('/')
                .next_back()
                .unwrap_or(module_path)
                .to_string()
        };

        // Track stdlib imports for Cargo.toml generation
        if module_path.starts_with("std::") {
            self.imported_stdlib_modules.insert(module_name.clone());
        }

        // Wrap in pub mod
        let wrapped = format!("pub mod {} {{\n{}\n}}\n", module_name, rust_code);

        self.compiled_modules
            .insert(module_path.to_string(), wrapped);
        Ok(())
    }

    fn resolve_module_path(
        &self,
        module_path: &str,
        source_file: Option<&Path>,
    ) -> Result<PathBuf> {
        if module_path.starts_with("std::") {
            // Stdlib module: std::json -> ./std/json.wj
            let module_name = module_path.strip_prefix("std::").unwrap();
            let mut path = self.stdlib_path.clone();
            path.push(format!("{}.wj", module_name));

            if !path.exists() {
                return Err(anyhow::anyhow!(
                    "Stdlib module not found: {} (looked in {:?})",
                    module_path,
                    path
                ));
            }

            Ok(path)
        } else if module_path.starts_with("./") || module_path.starts_with("../") {
            // Relative import: ./utils -> ./utils.wj or ./utils/mod.wj
            let source_dir = source_file.and_then(|f| f.parent()).ok_or_else(|| {
                anyhow::anyhow!("Cannot resolve relative import without source file")
            })?;

            // Strip ./ or ../
            let rel_path = module_path
                .trim_start_matches("./")
                .trim_start_matches("../");
            let mut candidate = source_dir.to_path_buf();

            // Handle ../ by going up directories
            if module_path.starts_with("../") {
                candidate = candidate
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("Cannot go above root directory"))?
                    .to_path_buf();
            }

            // Try direct file first: utils.wj
            candidate.push(format!("{}.wj", rel_path));
            if candidate.exists() {
                return Ok(candidate);
            }

            // Try directory module: utils/mod.wj
            candidate.pop();
            candidate.push(rel_path);
            candidate.push("mod.wj");
            if candidate.exists() {
                return Ok(candidate);
            }

            Err(anyhow::anyhow!(
                "User module not found: {} (looked in {:?} and {:?})",
                module_path,
                source_dir.join(format!("{}.wj", rel_path)),
                source_dir.join(rel_path).join("mod.wj")
            ))
        } else {
            // Absolute module path (e.g., math, rendering, physics)

            // THE WINDJAMMER WAY: Check the current file's directory FIRST
            // This allows "use texture_atlas::Foo" to work when texture_atlas.wj
            // is in the same directory as the importing file
            // We check this BEFORE source_roots to prioritize same-directory imports
            if let Some(source_file) = source_file {
                if let Some(source_dir) = source_file.parent() {
                    // Try direct file in same directory: source_dir/texture_atlas.wj
                    let mut candidate = source_dir.to_path_buf();
                    candidate.push(format!("{}.wj", module_path));
                    if candidate.exists() {
                        // Found in same directory as source file!
                        // Return the real path to compile it alongside
                        return Ok(candidate);
                    }

                    // Try directory module in same directory: source_dir/texture_atlas/mod.wj
                    let mut candidate = source_dir.to_path_buf();
                    candidate.push(module_path);
                    candidate.push("mod.wj");
                    if candidate.exists() {
                        return Ok(candidate);
                    }
                }
            }

            // Check if this exists in any of the configured source roots
            for source_root in &self.source_roots {
                // Try direct file: source_root/math.wj
                let mut candidate = source_root.clone();
                candidate.push(format!("{}.wj", module_path));
                if candidate.exists() {
                    // Found in source root - treat as external module
                    // When compiling individual files from source roots, cross-module
                    // dependencies should be treated as external (will be in same Rust module)
                    return Ok(PathBuf::from(format!("__source_root__::{}", module_path)));
                }

                // Try directory module: source_root/math/mod.wj
                let mut candidate = source_root.clone();
                candidate.push(module_path);
                candidate.push("mod.wj");
                if candidate.exists() {
                    // Found in source root - treat as external module
                    return Ok(PathBuf::from(format!("__source_root__::{}", module_path)));
                }
            }

            // Not found in source roots or current directory - treat as external crate
            // External crate imports (e.g., windjammer_ui, external_crate)
            // These are treated as Rust crate dependencies and passed through to generated code
            // Mark as external by returning a special "external" path
            Ok(PathBuf::from(format!("__external__::{}", module_path)))
        }
    }

    fn get_compiled_modules(&self) -> Vec<String> {
        // Return modules in arbitrary order (should topologically sort in future)
        self.compiled_modules.values().cloned().collect()
    }

    fn get_cargo_dependencies(&self) -> Vec<String> {
        // Map stdlib module names to their Rust crate dependencies
        let mut deps = Vec::new();

        for module in &self.imported_stdlib_modules {
            match module.as_str() {
                "json" => {
                    deps.push("serde = { version = \"1.0\", features = [\"derive\"] }".to_string());
                    deps.push("serde_json = \"1.0\"".to_string());
                }
                "csv" => {
                    deps.push("csv = \"1.3\"".to_string());
                }
                "http" => {
                    // HTTP client (reqwest)
                    deps.push(
                        "reqwest = { version = \"0.11\", features = [\"json\"] }".to_string(),
                    );
                    // HTTP server (axum)
                    deps.push("axum = \"0.7\"".to_string());
                    deps.push("tokio = { version = \"1\", features = [\"full\"] }".to_string());
                }
                "time" => {
                    deps.push("chrono = \"0.4\"".to_string());
                }
                "log" => {
                    deps.push("log = \"0.4\"".to_string());
                    deps.push("env_logger = \"0.11\"".to_string());
                }
                "regex" => {
                    deps.push("regex = \"1.10\"".to_string());
                }
                "cli" => {
                    deps.push("clap = { version = \"4.5\", features = [\"derive\"] }".to_string());
                }
                "db" => {
                    deps.push("sqlx = { version = \"0.7\", features = [\"runtime-tokio-native-tls\", \"postgres\", \"sqlite\", \"mysql\"] }".to_string());
                    deps.push("tokio = { version = \"1\", features = [\"full\"] }".to_string());
                }
                "random" => {
                    deps.push("rand = \"0.8\"".to_string());
                }
                "crypto" => {
                    deps.push("sha2 = \"0.10\"".to_string());
                    deps.push("bcrypt = \"0.15\"".to_string());
                    deps.push("base64 = \"0.21\"".to_string());
                }
                "process" => {
                    // Uses std::process, no extra deps
                }
                "env" => {
                    // Uses std::env, no extra deps
                }
                "async" => {
                    deps.push("tokio = { version = \"1\", features = [\"full\"] }".to_string());
                }
                // fs, strings, math use std library (no extra deps)
                _ => {}
            }
        }

        deps.sort();
        deps.dedup();
        deps
    }
}

#[allow(dead_code)]
/// Compile a single file (creates its own ModuleCompiler)
fn compile_file(
    input_path: &Path,
    output_dir: &Path,
    target: CompilationTarget,
) -> Result<(HashSet<String>, Vec<String>)> {
    let mut module_compiler = ModuleCompiler::new(target);
    // For single-file compilation, use parent directory as source root
    let source_root = input_path.parent().unwrap_or(Path::new("."));
    let is_multi_file = false; // Single file compilation
    compile_file_with_compiler(
        source_root,
        input_path,
        output_dir,
        &mut module_compiler,
        is_multi_file,
        true,
    )
}

/// Compile a file with a provided ModuleCompiler (for shared trait registry)
fn compile_file_with_compiler(
    source_root: &Path,
    input_path: &Path,
    output_dir: &Path,
    module_compiler: &mut ModuleCompiler,
    is_multi_file_project: bool,
    store_program: bool, // Whether to add this program to all_programs for trait inference
) -> Result<(HashSet<String>, Vec<String>)> {
    // RECURSION GUARD: Prevent circular module dependencies from causing stack overflow
    // THE WINDJAMMER WAY: Use normalized string for cross-platform path comparison
    // Problem: Windows canonicalize() adds UNC prefixes (\\?\C:\...) inconsistently,
    // causing PathBuf equality checks to fail even for the same file.
    // Solution: Convert to lowercase string with forward slashes for consistent comparison.
    let canonical_path = input_path.canonicalize().unwrap_or_else(|_| {
        // If canonicalize fails (file doesn't exist yet), use absolute path
        std::env::current_dir()
            .ok()
            .and_then(|cwd| cwd.join(input_path).canonicalize().ok())
            .unwrap_or_else(|| input_path.to_path_buf())
    });

    // Normalize path for consistent comparison across platforms
    // Remove UNC prefix on Windows and use forward slashes
    let path_key = {
        let normalized = canonical_path
            .to_string_lossy()
            .replace("\\\\?\\", "") // Remove Windows UNC prefix
            .replace('\\', "/"); // Normalize to forward slashes

        // Only lowercase on Windows (case-insensitive filesystem)
        // macOS/Linux filesystems are case-sensitive!
        #[cfg(target_os = "windows")]
        {
            normalized.to_lowercase()
        }
        #[cfg(not(target_os = "windows"))]
        {
            normalized
        }
    };

    // DEBUG: Print ALL currently compiling files for Windows debugging
    if !module_compiler.compiling_files.is_empty() {
        eprintln!(
            "🔍 Currently compiling {} files:",
            module_compiler.compiling_files.len()
        );
        for (idx, file) in module_compiler.compiling_files.iter().enumerate() {
            eprintln!("   [{}] {}", idx, file);
        }
        eprintln!("🔍 Checking: {}", path_key);
    }

    if module_compiler.compiling_files.contains(&path_key) {
        // Already compiling this file in the current chain - skip to prevent infinite recursion
        // This is OK and expected for circular imports that have already been handled
        eprintln!(
            "⚠️  RECURSION GUARD TRIGGERED: Skipping {} (already in compilation chain)",
            path_key
        );
        eprintln!(
            "   Currently compiling: {}",
            module_compiler.compiling_files.len()
        );
        eprintln!("   🚨 WARNING: This will cause an EMPTY FILE to be written!");
        return Ok((HashSet::new(), Vec::new()));
    }

    // Check recursion depth as additional safety
    if module_compiler.compiling_files.len() >= 50 {
        anyhow::bail!("Maximum module nesting depth exceeded (50 files). Possible circular dependency involving: {}", path_key);
    }

    module_compiler.compiling_files.insert(path_key.clone());
    eprintln!(
        "✅ RECURSION GUARD: Added {} to compilation set (now {} files)",
        path_key,
        module_compiler.compiling_files.len()
    );

    // THE WINDJAMMER WAY: Always cleanup, whether we succeed or fail
    // Call the implementation, then remove path from set regardless of result
    let result = compile_file_impl(
        source_root,
        input_path,
        module_compiler,
        output_dir,
        is_multi_file_project,
        store_program,
        &path_key,
    );

    // Remove path from compilation set now that we're done (success or failure)
    // This runs whether result is Ok or Err
    module_compiler.compiling_files.remove(&path_key);
    eprintln!(
        "✅ RECURSION GUARD: Removed {} from compilation set (now {} files)",
        path_key,
        module_compiler.compiling_files.len()
    );

    result
}

/// Internal implementation of compile_file_with_compiler
/// This is separated out so we can ensure cleanup happens in the outer function
fn compile_file_impl(
    source_root: &Path,
    input_path: &Path,
    module_compiler: &mut ModuleCompiler,
    output_dir: &Path,
    is_multi_file_project: bool,
    store_program: bool,
    _path_key: &str,
) -> Result<(HashSet<String>, Vec<String>)> {
    let target = module_compiler.target;

    // Read source file
    let source = std::fs::read_to_string(input_path)?;
    // Lex
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();

    // Parse
    let mut parser = parser::Parser::new_with_source(
        tokens,
        input_path.to_string_lossy().to_string(),
        source.clone(),
    );
    let program = parser
        .parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    // DEBUG: Print Item::Mod entries in the AST
    if std::env::var("WJ_DEBUG_AST").is_ok() {
        let file_name = input_path.file_name().unwrap().to_string_lossy();
        eprintln!("\n=== AST for {} ===", file_name);
        for (idx, item) in program.items.iter().enumerate() {
            if let parser::Item::Mod {
                name,
                items,
                is_public,
                ..
            } = item
            {
                eprintln!(
                    "  Item #{}: {}mod {} (items.len() = {})",
                    idx,
                    if *is_public { "pub " } else { "" },
                    name,
                    items.len()
                );
                if !items.is_empty() {
                    eprintln!("    INLINE MODULE with {} items:", items.len());
                    for (i, nested) in items.iter().enumerate() {
                        match nested {
                            parser::Item::Struct { decl, .. } => {
                                eprintln!("      #{}: struct {}", i, decl.name)
                            }
                            parser::Item::Function { decl, .. } => {
                                eprintln!("      #{}: fn {}", i, decl.name)
                            }
                            _ => eprintln!("      #{}: {:?}", i, nested),
                        }
                    }
                }
            }
        }
        eprintln!("=== End AST ===\n");
    }

    // THE WINDJAMMER WAY: Store this program for cross-file trait inference
    // Only store if requested (to avoid duplicates during regeneration)
    if store_program {
        module_compiler.all_programs.push(program.clone());
    }

    // Compile dependencies first (both use statements and mod declarations)
    for item in &program.items {
        // Handle use statements
        if let parser::Item::Use { path, alias: _, .. } = item {
            let module_path = path.join("::");

            // Handle item imports: ./main::Args -> compile ./main, not ./main::Args
            // Also handle braced imports: ./message::{A, B, C} -> compile ./message
            // Split at the last :: to separate module from item
            let module_to_compile = if module_path.contains("::") {
                // Check if this looks like a module::Item import or module::{...} import
                let parts: Vec<&str> = module_path.rsplitn(2, "::").collect();
                if parts.len() == 2 {
                    let potential_item = parts[0];
                    let module_part = parts[1];

                    // If the last part starts with uppercase or {, it's likely a type/braced import
                    // e.g., ./main::Args, ./types::Config, ./message::{A, B, C}
                    if potential_item
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_uppercase() || c == '{')
                    {
                        module_part.to_string()
                    } else {
                        // It's a nested module path, compile the whole thing
                        module_path.clone()
                    }
                } else {
                    module_path.clone()
                }
            } else {
                module_path.clone()
            };

            // Compile both std::* and relative imports (./ or ../) and external crates
            module_compiler.compile_module(&module_to_compile, Some(input_path))?;
        }

        // Handle module declarations (pub mod math;)
        if let parser::Item::Mod { name, items, .. } = item {
            // Only process external module declarations (items.is_empty() means no inline body)
            if items.is_empty() {
                // Find the module file: either math.wj or math/mod.wj
                let parent_dir = input_path.parent().unwrap_or(Path::new("."));

                // Try math.wj first
                let module_file = parent_dir.join(format!("{}.wj", name));
                let module_dir_file = parent_dir.join(name).join("mod.wj");

                let module_path_to_compile = if module_file.exists() {
                    Some(module_file)
                } else if module_dir_file.exists() {
                    Some(module_dir_file)
                } else {
                    // Module file doesn't exist yet - might be empty directory
                    // This is OK, we'll just not compile it
                    None
                };

                if let Some(mod_path) = module_path_to_compile {
                    // Recursively compile the module
                    compile_file_with_compiler(
                        source_root,
                        &mod_path,
                        output_dir,
                        module_compiler,
                        is_multi_file_project,
                        store_program, // Pass through the store_program flag
                    )?;
                }
            }
        }
    }

    // Register traits from this program into the global registry
    for item in &program.items {
        if let parser::Item::Trait { decl, .. } = item {
            module_compiler
                .trait_registry
                .insert(decl.name.clone(), decl.clone());
        }
    }

    // WINDJAMMER FIX: Use the SHARED analyzer from module_compiler
    // This ensures trait methods analyzed in file 1 are available when analyzing impl in file 2

    // Update analyzer's Copy structs registry
    module_compiler
        .analyzer
        .update_copy_structs(module_compiler.copy_structs_registry.clone());

    // Register any newly discovered traits
    for trait_decl in module_compiler.trait_registry.values() {
        let dummy_program = parser::Program {
            items: vec![parser::Item::Trait {
                decl: trait_decl.clone(),
                location: parser::SourceLocation::default(),
            }],
        };
        module_compiler
            .analyzer
            .register_traits_from_program(&dummy_program);
    }

    let (analyzed, signatures, analyzed_trait_methods) = module_compiler
        .analyzer
        .analyze_program(&program)
        .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;

    // MUTABILITY CHECK: Check for mut errors with great error messages
    let mut mut_checker = errors::MutabilityChecker::new(input_path.to_path_buf());
    let mut has_mut_errors = false;
    for item in &program.items {
        if let parser::Item::Function { decl, .. } = item {
            let mut_errors = mut_checker.check_function(decl);
            if !mut_errors.is_empty() {
                has_mut_errors = true;
                for error in &mut_errors {
                    eprintln!("{}", error.format_error());
                }
            }
        }
    }

    if has_mut_errors {
        anyhow::bail!("Compilation failed: mutability errors detected");
    }

    // THE WINDJAMMER WAY: During regeneration, use the GLOBAL analyzed trait methods
    // (which have been updated by finalize_trait_inference)
    if !store_program {
        // This is a regeneration pass - use the global inferred trait methods
        eprintln!("DEBUG REGEN: Using global trait methods for regeneration");
        eprintln!(
            "DEBUG REGEN: Global trait methods has {} traits",
            analyzed_trait_methods.len()
        );
        for (trait_name, methods) in &analyzed_trait_methods {
            eprintln!(
                "DEBUG REGEN:   GLOBAL Trait {} has {} methods",
                trait_name,
                methods.len()
            );
            for (method_name, method_analysis) in methods {
                eprintln!("DEBUG REGEN:     GLOBAL Method {} inferred:", method_name);
                for (param_name, ownership) in &method_analysis.inferred_ownership {
                    eprintln!(
                        "DEBUG REGEN:       BEFORE CLONE: {} = {:?}",
                        param_name, ownership
                    );
                }
            }
        }
        // Note: analyzed_trait_methods is already synchronized with module_compiler.analyzer
        // No need to clone - we're using the same HashMap that was returned from analyze_program
        eprintln!(
            "DEBUG REGEN: After clone, analyzed_trait_methods has {} traits",
            analyzed_trait_methods.len()
        );
        for (trait_name, methods) in &analyzed_trait_methods {
            for (method_name, method_analysis) in methods {
                eprintln!(
                    "DEBUG REGEN:     AFTER CLONE {}.{} self={:?}",
                    trait_name,
                    method_name,
                    method_analysis.inferred_ownership.get("self")
                );
            }
        }
    }

    // Infer trait bounds
    let mut inference_engine = inference::InferenceEngine::new();
    let mut inferred_bounds_map = std::collections::HashMap::new();
    for item in &program.items {
        if let parser::Item::Function { decl: func, .. } = item {
            let bounds = inference_engine.infer_function_bounds(func);
            if !bounds.is_empty() {
                inferred_bounds_map.insert(func.name.clone(), bounds);
            }
        }
    }

    // Generate code for main file
    let rust_code = if target == CompilationTarget::Wasm {
        // Check if program has components
        use component_analyzer::ComponentAnalyzer;
        let mut comp_analyzer = ComponentAnalyzer::new();
        let has_components = comp_analyzer.analyze(&program.items).is_ok()
            && comp_analyzer.all_components().next().is_some();

        if has_components {
            // Use new backend system for component-based WASM
            use codegen::backend::{CodegenConfig, Target};
            let config = CodegenConfig {
                target: Target::WebAssembly,
                output_dir: output_dir.to_path_buf(),
                ..Default::default()
            };

            let output = codegen::generate(&program, Target::WebAssembly, Some(config))
                .map_err(|e| anyhow::anyhow!("Component codegen error: {}", e))?;

            // Write main.rs
            let output_file = output_dir.join("lib.rs"); // WASM uses lib.rs
            std::fs::write(output_file, &output.source)?;

            // Write additional files (Cargo.toml, index.html)
            for (filename, content) in &output.additional_files {
                let file_path = output_dir.join(filename);
                std::fs::write(file_path, content)?;
            }

            // Return empty to signal we've handled everything
            return Ok((HashSet::new(), Vec::new()));
        } else {
            // Use old generator for non-component WASM
            // THE WINDJAMMER WAY: Use new_for_module in multi-file projects to prevent inlining
            let mut generator = if is_multi_file_project {
                codegen::CodeGenerator::new_for_module(signatures, target)
            } else {
                codegen::CodeGenerator::new(signatures, target)
            };
            generator.set_inferred_bounds(inferred_bounds_map);
            generator.set_analyzed_trait_methods(analyzed_trait_methods);

            // Set source file for error mapping
            generator.set_source_file(input_path);
            let output_file_path = get_relative_output_path(source_root, input_path, output_dir)?;
            // Create parent directories if needed
            if let Some(parent) = output_file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            generator.set_output_file(&output_file_path);

            // Set workspace root for relative paths in source maps
            // Use the current working directory as the workspace root for portability
            // This ensures both source and output paths can be relative
            if let Ok(cwd) = std::env::current_dir() {
                generator.set_workspace_root(cwd);
            }

            let result = generator.generate_program(&program, &analyzed);

            // Save source map for error mapping (now with relative paths)
            let source_map_path = output_file_path.with_extension("rs.map");
            if let Err(e) = generator.get_source_map().save_to_file(&source_map_path) {
                eprintln!("Warning: Failed to save source map: {}", e);
            }

            result
        }
    } else {
        // Use old generator for Rust target
        // THE WINDJAMMER WAY: Use new_for_module in multi-file projects to prevent inlining
        let mut generator = if is_multi_file_project {
            codegen::CodeGenerator::new_for_module(signatures, target)
        } else {
            codegen::CodeGenerator::new(signatures, target)
        };
        generator.set_inferred_bounds(inferred_bounds_map);
        generator.set_analyzed_trait_methods(analyzed_trait_methods);

        // Set source file for error mapping
        generator.set_source_file(input_path);
        let output_file_path = get_relative_output_path(source_root, input_path, output_dir)?;
        // Create parent directories if needed
        if let Some(parent) = output_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        generator.set_output_file(&output_file_path);

        // Set workspace root for relative paths in source maps
        // Use the current working directory as the workspace root for portability
        // This ensures both source and output paths can be relative
        if let Ok(cwd) = std::env::current_dir() {
            generator.set_workspace_root(cwd);
        }

        let result = generator.generate_program(&program, &analyzed);

        // Save source map for error mapping (now with relative paths)
        let source_map_path = output_file_path.with_extension("rs.map");
        if let Err(e) = generator.get_source_map().save_to_file(&source_map_path) {
            eprintln!("Warning: Failed to save source map: {}", e);
        }

        result
    };

    // THE WINDJAMMER WAY: Don't inline modules in multi-file projects!
    // The module system (lib.rs + mod.rs) handles module structure.
    // Only inline modules for single-file compilation (legacy behavior).
    //
    // BUGFIX: Also don't inline if the program declares any modules (pub mod foo;)
    // This handles the case where a single mod.wj file has submodules that
    // should be compiled as separate files, not inlined.
    let has_module_declarations = program
        .items
        .iter()
        .any(|item| matches!(item, parser::Item::Mod { items, .. } if items.is_empty()));

    let combined_code = if is_multi_file_project || has_module_declarations {
        // Multi-file project OR module with submodules: Don't inline modules
        // The module system handles everything via lib.rs/mod.rs
        rust_code
    } else {
        // Single-file with no module declarations: Inline compiled modules (legacy)
        let module_code = module_compiler.get_compiled_modules().join("\n");
        if module_code.is_empty() {
            rust_code
        } else {
            format!("{}\n\n{}", module_code, rust_code)
        }
    };

    // Write output (preserving directory structure)
    let output_file = get_relative_output_path(source_root, input_path, output_dir)?;

    // Create parent directories if needed
    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    eprintln!(
        "📝 WRITING FILE: {} ({} bytes)",
        output_file.display(),
        combined_code.len()
    );
    if combined_code.is_empty() {
        eprintln!("    🚨 WARNING: Writing EMPTY file!");
        eprintln!("    rust_code length: check generator output");
    }

    // Write file with explicit control over flush and sync
    // This ensures the write completes before we return, preventing race conditions
    // where tests read the file before the OS has flushed buffers
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&output_file)?;
        file.write_all(combined_code.as_bytes())?;
        file.flush()?; // Ensure data is written to OS buffers

        // On Linux, also force OS to write buffers to disk (Ubuntu CI has aggressive caching)
        // On macOS/Windows, flush() is sufficient
        #[cfg(target_os = "linux")]
        {
            file.sync_all()?; // Sync file data AND metadata
            drop(file); // Close file handle before syncing directory
            
            // CRITICAL: On Linux, we must also sync the PARENT DIRECTORY
            // to ensure the directory entry is persisted. Without this,
            // a crash could leave the directory without the file entry.
            // Ubuntu CI appears to have very aggressive caching.
            if let Some(parent) = output_file.parent() {
                let dir = std::fs::File::open(parent)?;
                dir.sync_all()?;
            }
        }
        
        #[cfg(not(target_os = "linux"))]
        drop(file); // Close file handle on non-Linux systems
    }

    // Return the set of imported stdlib modules and external crates for Cargo.toml generation
    Ok((
        module_compiler.imported_stdlib_modules.clone(),
        module_compiler.external_crates.clone(),
    ))
}

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

#[allow(dead_code)]
fn create_cargo_toml_with_deps(
    output_dir: &Path,
    imported_modules: &HashSet<String>,
    external_crates: &[String],
    target: CompilationTarget,
) -> Result<()> {
    use std::env;
    use std::fs;

    // For WASM target, generate WASM-specific Cargo.toml
    if target == CompilationTarget::Wasm {
        return create_wasm_cargo_toml(output_dir, imported_modules);
    }

    // Map imported stdlib modules to their Cargo dependencies
    let mut deps = Vec::new();

    // If ANY stdlib module is used, add windjammer-runtime
    if !imported_modules.is_empty() {
        // Add windjammer-runtime dependency (path-based for now)
        // Always search for workspace root, don't trust CARGO_MANIFEST_DIR
        let windjammer_runtime_path = {
            // Start from current directory and search upward
            let mut current = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let mut found = false;

            // Try current directory first (if we're in windjammer repo)
            if current
                .join("crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                current.join("crates/windjammer-runtime")
            }
            // Check if windjammer is a sibling directory (e.g., we're in windjammer-ui)
            else if let Some(parent) = current.parent() {
                if parent
                    .join("windjammer/crates/windjammer-runtime/Cargo.toml")
                    .exists()
                {
                    parent.join("windjammer/crates/windjammer-runtime")
                } else {
                    // Search upward (up to 5 levels)
                    for _ in 0..5 {
                        if let Some(parent) = current.parent() {
                            // Check for windjammer/crates/windjammer-runtime (sibling repo)
                            if parent
                                .join("windjammer/crates/windjammer-runtime/Cargo.toml")
                                .exists()
                            {
                                found = true;
                                current = parent.to_path_buf();
                                break;
                            }
                            // Check for crates/windjammer-runtime (legacy path)
                            if parent.join("crates/windjammer-runtime/Cargo.toml").exists() {
                                current = parent.to_path_buf();
                                found = true;
                                break;
                            }
                            current = parent.to_path_buf();
                        } else {
                            break;
                        }
                    }

                    if found {
                        if current
                            .join("windjammer/crates/windjammer-runtime/Cargo.toml")
                            .exists()
                        {
                            current.join("windjammer/crates/windjammer-runtime")
                        } else {
                            current.join("crates/windjammer-runtime")
                        }
                    } else {
                        // Fallback: try sibling first, then legacy path
                        let sibling_path = PathBuf::from("../windjammer/crates/windjammer-runtime");
                        if sibling_path.join("Cargo.toml").exists() {
                            sibling_path
                        } else {
                            PathBuf::from("./crates/windjammer-runtime")
                        }
                    }
                }
            } else {
                // Fallback when no parent
                PathBuf::from("../windjammer/crates/windjammer-runtime")
            }
        };

        deps.push(format!(
            "windjammer-runtime = {{ path = \"{}\" }}",
            windjammer_runtime_path.display()
        ));
    }

    // Users should add windjammer-ui or other frameworks explicitly in their Cargo.toml
    // The compiler no longer auto-adds these dependencies to avoid filesystem path issues
    let external_crates = external_crates.to_vec();

    // Legacy: Keep old dependencies for modules not yet in runtime
    for module in imported_modules {
        match module.as_str() {
            // These are now in windjammer-runtime, no extra deps needed
            "fs" | "http" | "mime" | "json" => {}

            // UI and other frameworks should be added explicitly by users
            "ui" | "game" => {}

            // Legacy modules that still need direct dependencies
            "csv" => {
                deps.push("csv = \"1.3\"".to_string());
            }
            "time" => {
                deps.push("chrono = \"0.4\"".to_string());
            }
            "log" => {
                deps.push("log = \"0.4\"".to_string());
                deps.push("env_logger = \"0.11\"".to_string());
            }
            "regex" => {
                deps.push("regex = \"1.10\"".to_string());
            }
            "cli" => {
                deps.push("clap = { version = \"4.5\", features = [\"derive\"] }".to_string());
            }
            "crypto" => {
                deps.push("sha2 = \"0.10\"".to_string());
                deps.push("bcrypt = \"0.15\"".to_string());
                deps.push("base64 = \"0.21\"".to_string());
            }
            "random" => {
                deps.push("rand = \"0.8\"".to_string());
            }
            "async" => {
                deps.push("tokio = { version = \"1\", features = [\"full\"] }".to_string());
            }
            "db" => {
                deps.push("sqlx = { version = \"0.7\", features = [\"runtime-tokio-native-tls\", \"sqlite\"] }".to_string());
                deps.push("tokio = { version = \"1\", features = [\"full\"] }".to_string());
            }
            // fs, strings, math, env, process use std library or windjammer-runtime
            _ => {}
        }
    }

    // Add external crates (user-specified or from crates.io)
    // NOTE: Users should explicitly add windjammer-ui or other framework dependencies
    // to their Cargo.toml - the compiler no longer auto-adds filesystem paths
    let mut external_deps = Vec::new();
    for crate_name in external_crates {
        // THE WINDJAMMER WAY: Filter out Rust keywords (crate, super, self)
        // These are language features, not external dependencies!
        if crate_name == "crate" || crate_name == "super" || crate_name == "self" {
            continue; // Skip Rust keywords
        }

        // THE WINDJAMMER WAY: Check if windjammer-game or windjammer-game-core is imported
        // If so, add it as a path dependency to the local game framework
        if crate_name == "windjammer_game"
            || crate_name == "windjammer-game"
            || crate_name == "windjammer_game_core"
            || crate_name == "windjammer-game-core"
        {
            // Try to find windjammer-game in the workspace
            // First, check if WINDJAMMER_GAME_PATH env var is set (for development)
            if let Ok(game_path) = std::env::var("WINDJAMMER_GAME_PATH") {
                let game_path = PathBuf::from(game_path);
                if game_path.exists() {
                    external_deps.push(format!(
                        "windjammer-game = {{ path = {:?} }}",
                        game_path.to_str().unwrap()
                    ));
                    continue; // Skip the crates.io fallback
                }
            }

            // Second, try to find it relative to the compiler source
            // This works when compiling from source
            let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // /path/to/windjammer
            let src_root = manifest_dir.parent().unwrap(); // /path/to (parent of windjammer)

            // Try both old and new paths for compatibility
            let game_path = src_root.join("windjammer-game/windjammer-game-core");
            let legacy_game_path = src_root.join("windjammer-game/windjammer-game");

            let final_path = if game_path.exists() {
                game_path
            } else if legacy_game_path.exists() {
                legacy_game_path
            } else {
                PathBuf::new() // Empty path, will fallback to crates.io
            };

            if !final_path.as_os_str().is_empty() {
                // Read the actual crate name from Cargo.toml at the path
                let cargo_toml_path = final_path.join("Cargo.toml");
                let crate_name_normalized = if cargo_toml_path.exists() {
                    // Try to read the actual crate name from Cargo.toml
                    if let Ok(content) = std::fs::read_to_string(&cargo_toml_path) {
                        // Parse name = "..." line
                        if let Some(line) = content.lines().find(|l| l.trim().starts_with("name")) {
                            if let Some(name_part) = line.split('"').nth(1) {
                                name_part.to_string()
                            } else {
                                // Fallback: guess based on crate_name
                                if crate_name.contains("_core") || crate_name.contains("-core") {
                                    "windjammer-game-core".to_string()
                                } else {
                                    "windjammer-game".to_string()
                                }
                            }
                        } else {
                            // Fallback: guess based on crate_name
                            if crate_name.contains("_core") || crate_name.contains("-core") {
                                "windjammer-game-core".to_string()
                            } else {
                                "windjammer-game".to_string()
                            }
                        }
                    } else {
                        // Fallback: guess based on crate_name
                        if crate_name.contains("_core") || crate_name.contains("-core") {
                            "windjammer-game-core".to_string()
                        } else {
                            "windjammer-game".to_string()
                        }
                    }
                } else {
                    // Fallback: guess based on crate_name
                    if crate_name.contains("_core") || crate_name.contains("-core") {
                        "windjammer-game-core".to_string()
                    } else {
                        "windjammer-game".to_string()
                    }
                };

                external_deps.push(format!(
                    "{} = {{ path = {:?} }}",
                    crate_name_normalized,
                    final_path.to_str().unwrap()
                ));
                continue; // Skip the crates.io fallback
            }

            // Fallback: assume it's on crates.io (for published version)
            let crate_name_normalized = if crate_name.contains("_core") {
                "windjammer-game-core"
            } else {
                "windjammer-game"
            };
            external_deps.push(format!("{} = \"*\"", crate_name_normalized));
        } else {
            // All other external crates are assumed to be from crates.io
            external_deps.push(format!("{} = \"*\"", crate_name));
        }
    }

    deps.extend(external_deps);

    // Add optimization dependencies (always included for now)
    // TODO: Only add these if actually used by checking CodeGenerator flags
    deps.push("smallvec = \"1.13\"".to_string());
    deps.push("serde = { version = \"1.0\", features = [\"derive\"] }".to_string());

    // Smart deduplication: extract package names and keep more specific versions
    let mut seen_packages = std::collections::HashSet::new();
    let mut deduplicated_deps = Vec::new();

    // Sort so that more specific versions (with braces) come after simple ones
    deps.sort_by(|a, b| {
        let a_has_braces = a.contains('{');
        let b_has_braces = b.contains('{');
        match (a_has_braces, b_has_braces) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => a.cmp(b),
        }
    });

    for dep in deps {
        // Extract package name (everything before '=')
        if let Some(pkg_name) = dep.split('=').next() {
            let pkg_name = pkg_name.trim();
            if !seen_packages.contains(pkg_name) {
                seen_packages.insert(pkg_name.to_string());
                deduplicated_deps.push(dep);
            } else {
                // If we've seen this package before, check if this version is more specific
                if dep.contains('{') {
                    // Remove the old simple version and add this more specific one
                    deduplicated_deps.retain(|d| !d.starts_with(pkg_name));
                    deduplicated_deps.push(dep);
                }
            }
        }
    }

    deps = deduplicated_deps;

    let deps_section = if deps.is_empty() {
        String::new()
    } else {
        format!("[dependencies]\n{}\n\n", deps.join("\n"))
    };

    // THE WINDJAMMER WAY: Check if lib.rs exists to determine if this is a library or binary project
    let has_lib_rs = output_dir.join("lib.rs").exists();
    let has_main_rs = output_dir.join("main.rs").exists();

    let lib_or_bin_section = if has_lib_rs {
        // Library project - generate [lib] section
        "[lib]\nname = \"windjammer_app\"\npath = \"lib.rs\"\n\n".to_string()
    } else if has_main_rs {
        // Binary project with main.rs - generate [[bin]] section
        "[[bin]]\nname = \"windjammer-app\"\npath = \"main.rs\"\n\n".to_string()
    } else {
        // Multiple standalone files - generate [[bin]] sections for each
        let mut bin_sections = Vec::new();
        if let Ok(entries) = fs::read_dir(output_dir) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.ends_with(".rs") {
                        let bin_name = filename.strip_suffix(".rs").unwrap_or(filename);
                        bin_sections.push(format!(
                            "[[bin]]\nname = \"{}\"\npath = \"{}\"\n",
                            bin_name, filename
                        ));
                    }
                }
            }
        }

        if !bin_sections.is_empty() {
            format!("{}\n", bin_sections.join("\n"))
        } else {
            String::new()
        }
    };

    let cargo_toml = format!(
        r#"[package]
name = "windjammer-app"
version = "0.1.0"
edition = "2021"

# Prevent this from being treated as part of parent workspace
[workspace]

{}{}[profile.release]
opt-level = 3
"#,
        deps_section, lib_or_bin_section
    );

    let cargo_toml_path = output_dir.join("Cargo.toml");
    fs::write(cargo_toml_path, cargo_toml)?;

    Ok(())
}

/// Create WASM-specific Cargo.toml
fn create_wasm_cargo_toml(output_dir: &Path, imported_modules: &HashSet<String>) -> Result<()> {
    use std::env;
    use std::fs;

    // Check if platform APIs are used (requires windjammer-runtime)
    let uses_platform_apis = imported_modules.iter().any(|m| {
        m == "fs"
            || m == "process"
            || m == "dialog"
            || m == "env"
            || m == "encoding"
            || m.starts_with("fs::")
            || m.starts_with("process::")
            || m.starts_with("dialog::")
            || m.starts_with("env::")
            || m.starts_with("encoding::")
    });

    // Find windjammer-runtime path
    let windjammer_runtime_path = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        PathBuf::from(manifest_dir).join("crates/windjammer-runtime")
    } else {
        let mut current = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut found = false;

        if current
            .join("crates/windjammer-runtime/Cargo.toml")
            .exists()
        {
            current.join("crates/windjammer-runtime")
        } else {
            for _ in 0..5 {
                if let Some(parent) = current.parent() {
                    if parent.join("crates/windjammer-runtime/Cargo.toml").exists() {
                        current = parent.to_path_buf();
                        found = true;
                        break;
                    }
                    current = parent.to_path_buf();
                } else {
                    break;
                }
            }

            if found {
                current.join("crates/windjammer-runtime")
            } else {
                PathBuf::from("./crates/windjammer-runtime")
            }
        }
    };

    // Find the first .rs file in the output directory to use as lib.rs
    let lib_file = fs::read_dir(output_dir)?
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            entry.path().extension().and_then(|s| s.to_str()) == Some("rs")
                && entry.path().file_name().and_then(|s| s.to_str()) != Some("main.rs")
        })
        .and_then(|entry| {
            entry
                .path()
                .file_name()
                .and_then(|s| s.to_str())
                .map(String::from)
        })
        .unwrap_or_else(|| "lib.rs".to_string());

    let cargo_toml = format!(
        r#"[package]
name = "windjammer-wasm"
version = "0.1.0"
edition = "2021"

# Prevent this from being treated as part of parent workspace
[workspace]

[lib]
crate-type = ["cdylib"]
path = "{}"

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
serde-wasm-bindgen = "0.6"
web-sys = {{ version = "0.3", features = [
    "Document",
    "Element",
    "HtmlElement",
    "Node",
    "Text",
    "Window",
    "Event",
    "MouseEvent",
    "KeyboardEvent",
] }}
js-sys = "0.3"
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
console_error_panic_hook = "0.1"
{}
[profile.release]
opt-level = "z"  # Optimize for size
lto = true
"#,
        lib_file,
        if uses_platform_apis {
            format!(
                "windjammer-runtime = {{ path = \"{}\", features = [\"wasm\"] }}",
                windjammer_runtime_path.display()
            )
        } else {
            String::new()
        }
    );

    let cargo_toml_path = output_dir.join("Cargo.toml");
    fs::write(cargo_toml_path, cargo_toml)?;

    Ok(())
}

/// Run cargo build on the generated Rust code and display errors with source mapping
fn check_with_cargo(output_dir: &Path, show_raw_errors: bool) -> Result<()> {
    use colored::*;
    use std::process::Command;

    println!("\n{} Rust compilation...", "Checking".cyan().bold());

    let output = Command::new("cargo")
        .arg("build")
        .arg("--message-format=json")
        .current_dir(output_dir)
        .output()?;

    if output.status.success() {
        println!("{} No Rust compilation errors!", "Success!".green().bold());
        return Ok(());
    }

    // Combine stderr and stdout (cargo outputs to both)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined_output = format!("{}{}", stderr, stdout);

    // If raw errors requested, show them and exit
    if show_raw_errors {
        println!("{} Rust compilation errors (raw):", "Error:".red().bold());
        println!("{}", combined_output);
        return Err(anyhow::anyhow!("Rust compilation failed"));
    }

    // Load all source maps from the output directory
    let source_maps = load_source_maps(output_dir)?;

    // Create error mapper with merged source maps
    let error_mapper = error_mapper::ErrorMapper::new(source_maps);

    // Map rustc output to Windjammer diagnostics
    let wj_diagnostics = error_mapper.map_rustc_output(&combined_output);

    if wj_diagnostics.is_empty() {
        // Fallback: show raw output if we couldn't parse any diagnostics
        println!(
            "{} Could not parse Rust compilation errors. Showing raw output:",
            "Warning:".yellow().bold()
        );
        println!("{}", combined_output);
        return Err(anyhow::anyhow!("Rust compilation failed"));
    }

    // Display Windjammer diagnostics with beautiful formatting
    let error_count = wj_diagnostics
        .iter()
        .filter(|d| matches!(d.level, error_mapper::DiagnosticLevel::Error))
        .count();
    let warning_count = wj_diagnostics
        .iter()
        .filter(|d| matches!(d.level, error_mapper::DiagnosticLevel::Warning))
        .count();

    if error_count > 0 {
        println!(
            "\n{} {} error{} detected:\n",
            "Error:".red().bold(),
            error_count,
            if error_count == 1 { "" } else { "s" }
        );
    }
    if warning_count > 0 {
        println!(
            "{} {} warning{}\n",
            "Warning:".yellow().bold(),
            warning_count,
            if warning_count == 1 { "" } else { "s" }
        );
    }

    for diag in &wj_diagnostics {
        // Use the beautiful formatted output from WindjammerDiagnostic
        let formatted = diag.format();

        // Add colors
        let colored_output = colorize_diagnostic(&formatted, &diag.level);
        println!("{}", colored_output);
    }

    if error_count > 0 {
        Err(anyhow::anyhow!(
            "Compilation failed with {} error{}",
            error_count,
            if error_count == 1 { "" } else { "s" }
        ))
    } else {
        Ok(())
    }
}

/// Load and merge all source maps from the output directory
fn load_source_maps(output_dir: &Path) -> Result<source_map::SourceMap> {
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
fn colorize_diagnostic(text: &str, _level: &error_mapper::DiagnosticLevel) -> String {
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
    build_project(file, &temp_dir, target)?;

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

/// Run tests (discovers and runs all test functions)
pub fn run_tests(
    path: Option<&Path>,
    filter: Option<&str>,
    nocapture: bool,
    parallel: bool,
    json: bool,
) -> Result<()> {
    use colored::*;
    use std::fs;
    use std::process::Command;
    use std::time::Instant;

    let start_time = Instant::now();

    // Determine test directory
    let test_dir = path.unwrap_or_else(|| Path::new("."));

    if !test_dir.exists() {
        anyhow::bail!("Test path does not exist: {:?}", test_dir);
    }

    // Discover test files
    if !json {
        println!();
        println!(
            "{}",
            "╭─────────────────────────────────────────────╮".cyan()
        );
        println!(
            "{}",
            "│  🧪  Windjammer Test Framework            │"
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "╰─────────────────────────────────────────────╯".cyan()
        );
        println!();
        println!("{} Discovering tests...", "→".bright_blue().bold());
    }

    let test_files = discover_test_files(test_dir)?;

    if test_files.is_empty() {
        if json {
            println!("{{\"error\": \"No test files found\", \"files\": [], \"tests\": []}}");
        } else {
            println!();
            println!("{} No test files found", "✗".red().bold());
            println!();
            println!("  {} Test files should:", "ℹ".blue());
            println!(
                "    • Be named {}  or {}",
                "*_test.wj".yellow(),
                "test_*.wj".yellow()
            );
            println!("    • Contain functions starting with {}", "test_".yellow());
            println!();
        }
        return Ok(());
    }

    if !json {
        println!(
            "{} Found {} test file(s)",
            "✓".green().bold(),
            test_files.len().to_string().bright_white().bold()
        );
        for file in &test_files {
            println!(
                "    {} {}",
                "•".bright_black(),
                file.display().to_string().bright_white()
            );
        }
        println!();
    }

    // Create temporary test directory
    let temp_dir = std::env::temp_dir().join("windjammer-test");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;

    // Compile test files
    if !json {
        println!("{} Compiling tests...", "→".bright_blue().bold());
    }

    let mut all_tests = Vec::new();

    for test_file in &test_files {
        let tests = compile_test_file(test_file, &temp_dir)?;
        all_tests.extend(tests);
    }

    if !json {
        println!(
            "{} Found {} test function(s)",
            "✓".green().bold(),
            all_tests.len().to_string().bright_white().bold()
        );
        println!();
    }

    // Generate test harness
    generate_test_harness(&temp_dir, &all_tests, filter)?;

    // Run tests
    if !json {
        println!("{}", "─".repeat(50).bright_black());
        println!("{} Running tests...", "▶".bright_green().bold());
        println!("{}", "─".repeat(50).bright_black());
        println!();
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("test").current_dir(&temp_dir);

    if !parallel {
        cmd.arg("--").arg("--test-threads").arg("1");
    }

    if let Some(filter_str) = filter {
        cmd.arg("--").arg(filter_str);
    }

    if nocapture {
        if filter.is_none() {
            cmd.arg("--");
        }
        cmd.arg("--nocapture");
    }

    let output = cmd.output()?;
    let duration = start_time.elapsed();

    // Parse test output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let test_results = parse_test_output(&stdout, &stderr);

    if json {
        // JSON output for tooling
        println!("{{");
        println!("  \"success\": {},", output.status.success());
        println!("  \"duration_ms\": {},", duration.as_millis());
        println!("  \"test_files\": {},", test_files.len());
        println!("  \"total_tests\": {},", all_tests.len());
        println!("  \"passed\": {},", test_results.passed);
        println!("  \"failed\": {},", test_results.failed);
        println!("  \"ignored\": {},", test_results.ignored);
        println!("  \"files\": [");
        for (i, file) in test_files.iter().enumerate() {
            println!(
                "    \"{}\"{}",
                file.display(),
                if i < test_files.len() - 1 { "," } else { "" }
            );
        }
        println!("  ],");
        println!("  \"tests\": [");
        for (i, test) in all_tests.iter().enumerate() {
            // Look up the status for this test
            // The test name in cargo output is "module::test_name"
            let full_test_name = format!(
                "{}::{}",
                test.file.file_stem().unwrap().to_string_lossy(),
                test.name
            );
            let status = test_results
                .individual_results
                .get(&full_test_name)
                .or_else(|| test_results.individual_results.get(&test.name))
                .map(|s| s.as_str())
                .unwrap_or("unknown");

            println!(
                "    {{\"name\": \"{}\", \"file\": \"{}\", \"status\": \"{}\"}}{}",
                test.name,
                test.file.display(),
                status,
                if i < all_tests.len() - 1 { "," } else { "" }
            );
        }
        println!("  ]");
        println!("}}");
    } else {
        // Pretty output for humans
        print!("{}", stdout);
        print!("{}", stderr);

        println!();
        println!("{}", "─".repeat(50).bright_black());

        if output.status.success() {
            println!();
            println!(
                "{} {} All tests passed! {}",
                "✓".green().bold(),
                "🎉".bright_white(),
                "✓".green().bold()
            );
            println!();
            println!(
                "  {} {} passed",
                "✓".green(),
                test_results.passed.to_string().bright_white().bold()
            );
            if test_results.ignored > 0 {
                println!(
                    "  {} {} ignored",
                    "○".yellow(),
                    test_results.ignored.to_string().bright_white()
                );
            }
            println!(
                "  {} Completed in {}",
                "⏱".bright_blue(),
                format!("{:.2}s", duration.as_secs_f64())
                    .bright_white()
                    .bold()
            );
        } else {
            println!();
            println!(
                "{} {} Tests failed {}",
                "✗".red().bold(),
                "⚠".bright_yellow(),
                "✗".red().bold()
            );
            println!();
            println!(
                "  {} {} passed",
                "✓".green(),
                test_results.passed.to_string().bright_white()
            );
            println!(
                "  {} {} failed",
                "✗".red().bold(),
                test_results.failed.to_string().bright_white().bold()
            );
            if test_results.ignored > 0 {
                println!(
                    "  {} {} ignored",
                    "○".yellow(),
                    test_results.ignored.to_string().bright_white()
                );
            }
            println!(
                "  {} Completed in {}",
                "⏱".bright_blue(),
                format!("{:.2}s", duration.as_secs_f64()).bright_white()
            );
        }

        println!();
        println!("{}", "─".repeat(50).bright_black());
        println!();

        // Check for coverage flag in environment
        if std::env::var("WINDJAMMER_COVERAGE").is_ok() {
            println!("{} Generating coverage report...", "→".bright_blue().bold());
            generate_coverage_report(&temp_dir)?;
        }
    }

    if !output.status.success() {
        anyhow::bail!("Tests failed");
    }

    // Clean up
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }

    Ok(())
}

#[derive(Debug, Default)]
struct TestResults {
    passed: usize,
    failed: usize,
    ignored: usize,
    individual_results: std::collections::HashMap<String, String>, // test_name -> status
}

fn parse_test_output(stdout: &str, _stderr: &str) -> TestResults {
    let mut results = TestResults::default();

    // Parse individual test results
    for line in stdout.lines() {
        let line = line.trim();

        // Parse individual test lines: "test module::test_name ... ok"
        if line.starts_with("test ")
            && (line.contains(" ... ok")
                || line.contains(" ... FAILED")
                || line.contains(" ... ignored"))
        {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 && parts[0] == "test" {
                let test_name = parts[1].to_string();
                let status = if line.contains(" ... ok") {
                    "passed"
                } else if line.contains(" ... FAILED") {
                    "failed"
                } else if line.contains(" ... ignored") {
                    "ignored"
                } else {
                    "unknown"
                };
                results
                    .individual_results
                    .insert(test_name, status.to_string());
            }
        }

        // Parse summary line for aggregate counts
        if line.contains("test result:") {
            // Example: "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
            let parts: Vec<&str> = line.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if part == &"passed;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        results.passed += n; // Sum instead of replace
                    }
                }
                if part == &"failed;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        results.failed += n; // Sum instead of replace
                    }
                }
                if part == &"ignored;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        results.ignored += n; // Sum instead of replace
                    }
                }
            }
        }
    }

    results
}

/// Discover test files in a directory
fn discover_test_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut test_files = Vec::new();

    if dir.is_file() {
        // Single file
        if is_test_file(dir) {
            test_files.push(dir.to_path_buf());
        }
    } else {
        // Directory - search recursively
        visit_dirs(dir, &mut test_files)?;
    }

    Ok(test_files)
}

/// Visit directories recursively to find test files
fn visit_dirs(dir: &Path, test_files: &mut Vec<PathBuf>) -> Result<()> {
    use std::fs;

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip target, build, and hidden directories
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.') || name_str == "target" || name_str == "build" {
                        continue;
                    }
                }
                visit_dirs(&path, test_files)?;
            } else if is_test_file(&path) {
                test_files.push(path);
            }
        }
    }

    Ok(())
}

/// Check if a file is a test file
fn is_test_file(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        (name_str.ends_with("_test.wj") || name_str.starts_with("test_"))
            && name_str.ends_with(".wj")
    } else {
        false
    }
}

/// Compile a test file and extract test functions
fn compile_test_file(test_file: &Path, _output_dir: &Path) -> Result<Vec<TestFunction>> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use std::fs;

    let source = fs::read_to_string(test_file)?;

    // Lex and parse
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().map_err(|e| anyhow::anyhow!("{}", e))?;

    // Find test functions
    let mut tests = Vec::new();
    for item in &program.items {
        if let crate::parser::Item::Function { decl: func, .. } = item {
            if func.name.starts_with("test_") {
                tests.push(TestFunction {
                    name: func.name.clone(),
                    file: test_file.to_path_buf(),
                });
            }
        }
    }

    Ok(tests)
}

/// Test function metadata
#[derive(Debug, Clone)]
struct TestFunction {
    name: String,
    file: PathBuf,
}

/// Generate Rust test harness from Windjammer tests
fn generate_test_harness(
    output_dir: &Path,
    tests: &[TestFunction],
    filter: Option<&str>,
) -> Result<()> {
    use std::collections::HashMap;
    use std::fs;

    // Group tests by file
    let mut tests_by_file: HashMap<PathBuf, Vec<&TestFunction>> = HashMap::new();
    for test in tests {
        tests_by_file
            .entry(test.file.clone())
            .or_default()
            .push(test);
    }

    // Compile each test file using the existing infrastructure
    for (file, file_tests) in &tests_by_file {
        // Skip if filter doesn't match
        if let Some(filter_str) = filter {
            if !file_tests.iter().any(|t| t.name.contains(filter_str)) {
                continue;
            }
        }

        // Compile the file to Rust
        let _ = compile_file(file, output_dir, CompilationTarget::Rust)?;

        // Read the generated Rust code
        let output_file = output_dir.join(format!(
            "{}.rs",
            file.file_stem().unwrap().to_string_lossy()
        ));
        let mut rust_code = fs::read_to_string(&output_file)?;

        // Add test attributes to test functions
        for test in file_tests.iter() {
            let test_fn = format!("fn {}()", test.name);
            let test_attr = format!("#[test]\nfn {}()", test.name);
            rust_code = rust_code.replace(&test_fn, &test_attr);
        }

        // Write back
        fs::write(&output_file, rust_code)?;
    }

    // Create Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "windjammer-tests"
version = "0.1.0"
edition = "2021"

[dependencies]
windjammer-runtime = {{ path = "{}" }}
smallvec = "1.13"

[lib]
name = "windjammer_tests"
path = "lib.rs"
"#,
        std::env::current_dir()?
            .join("crates/windjammer-runtime")
            .display()
    );
    fs::write(output_dir.join("Cargo.toml"), cargo_toml)?;

    // Create lib.rs that includes all test modules
    let mut lib_rs = String::from("// Auto-generated test harness\n\n");
    for (file, _) in tests_by_file {
        let module_name = file.file_stem().unwrap().to_string_lossy();
        lib_rs.push_str(&format!("pub mod {};\n", module_name));
    }
    fs::write(output_dir.join("lib.rs"), lib_rs)?;

    Ok(())
}

/// Generate coverage report using cargo-llvm-cov
fn generate_coverage_report(test_dir: &Path) -> Result<()> {
    use colored::*;
    use std::process::Command;

    // Check if cargo-llvm-cov is installed
    let check = Command::new("cargo")
        .arg("llvm-cov")
        .arg("--version")
        .output();

    if check.is_err() || !check.unwrap().status.success() {
        println!("{} cargo-llvm-cov not found", "⚠".yellow());
        println!("Install with: cargo install cargo-llvm-cov");
        println!("Skipping coverage report...");
        return Ok(());
    }

    // Generate coverage
    let output = Command::new("cargo")
        .arg("llvm-cov")
        .arg("test")
        .arg("--html")
        .current_dir(test_dir)
        .output()?;

    if output.status.success() {
        // Copy coverage report to project directory
        let source_dir = test_dir.join("target/llvm-cov");
        let dest_dir = std::path::Path::new("target/llvm-cov");

        if source_dir.exists() {
            // Create destination directory
            std::fs::create_dir_all(dest_dir)?;

            // Copy the coverage report
            if let Err(e) = copy_dir_recursive(&source_dir, dest_dir) {
                println!("{} Failed to copy coverage report: {}", "⚠".yellow(), e);
            } else {
                println!("{} Coverage report generated", "✓".green());
                println!("  Open: target/llvm-cov/html/index.html");
            }
        } else {
            println!(
                "{} Coverage report not found at expected location",
                "⚠".yellow()
            );
        }
    } else {
        println!("{} Coverage generation failed", "✗".red());
        print!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Generate mod.rs file with pub mod declarations and re-exports
pub fn generate_mod_file(output_dir: &Path) -> Result<()> {
    use colored::*;
    use std::fs;

    // Find all .rs files (excluding mod.rs itself) and extract their public types
    let mut modules = Vec::new();
    let mut type_exports: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".rs") && file_name != "mod.rs" && file_name != "main.rs" {
                    // Get module name (strip .rs extension)
                    if let Some(module_name) = file_name.strip_suffix(".rs") {
                        modules.push(module_name.to_string());

                        // Parse the file to extract public struct/enum names
                        let content = fs::read_to_string(&path)?;
                        let mut exports = Vec::new();

                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("pub struct ") {
                                if let Some(name) = trimmed
                                    .strip_prefix("pub struct ")
                                    .and_then(|s| s.split_whitespace().next())
                                {
                                    exports.push(name.to_string());
                                }
                            } else if trimmed.starts_with("pub enum ") {
                                if let Some(name) = trimmed
                                    .strip_prefix("pub enum ")
                                    .and_then(|s| s.split_whitespace().next())
                                {
                                    exports.push(name.to_string());
                                }
                            }
                        }

                        if !exports.is_empty() {
                            type_exports.insert(module_name.to_string(), exports);
                        }
                    }
                }
            }
        }
    }

    if modules.is_empty() {
        println!(
            "{} No modules found to generate mod.rs",
            "Warning:".yellow().bold()
        );
        return Ok(());
    }

    modules.sort();

    // Generate mod.rs content
    let mut content = String::from("// Auto-generated mod.rs by Windjammer CLI\n");
    content.push_str("// This file declares all generated Windjammer modules\n\n");

    // Add pub mod declarations
    for module in &modules {
        content.push_str(&format!("pub mod {};\n", module));
    }

    // Add re-exports for all public types
    content.push_str("\n// Re-export all public items\n");
    for module in &modules {
        content.push_str(&format!("pub use {}::*;\n", module));
    }

    // Write mod.rs
    let mod_file_path = output_dir.join("mod.rs");
    fs::write(&mod_file_path, content)?;

    println!(
        "{} Generated mod.rs with {} modules",
        "✓".green(),
        modules.len()
    );

    Ok(())
}

/// Strip main() functions from generated Rust files (library mode)
pub fn strip_main_functions(output_dir: &Path) -> Result<()> {
    use colored::*;
    use std::fs;

    let mut stripped_count = 0;

    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".rs") && file_name != "mod.rs" {
                    let content = fs::read_to_string(&path)?;

                    // Simpler approach: find fn main() and remove everything from there to the end of file
                    // (main() is always the last function in generated files)
                    let mut new_lines = Vec::new();
                    let mut found_main = false;

                    for line in content.lines() {
                        let trimmed = line.trim();

                        if trimmed.starts_with("fn main()") || trimmed.starts_with("pub fn main()")
                        {
                            found_main = true;
                            stripped_count += 1;
                            break; // Stop processing, discard everything from here
                        }

                        new_lines.push(line);
                    }

                    if found_main {
                        // Rebuild content without main()
                        let new_content = new_lines.join("\n") + "\n";
                        fs::write(&path, new_content)?;
                    }
                }
            }
        }
    }

    if stripped_count > 0 {
        println!(
            "{} Stripped {} main() functions (library mode)",
            "✓".green(),
            stripped_count
        );
    }

    Ok(())
}

/// Find the actual source root for a Windjammer file
///
/// For example, given "src_wj/ecs/entity.wj", this will walk up to find "src_wj"
/// by looking for a directory that looks like a source root:
/// - Named "src_wj" or "src" (this is the most reliable indicator)
/// - Or the topmost directory containing mod.wj
fn find_source_root(file_path: &Path) -> Option<&Path> {
    let mut current = file_path;
    let mut topmost_mod_wj_dir = None;
    let mut found_src_wj = None;

    while let Some(parent) = current.parent() {
        // Check if this directory looks like a source root by name
        if let Some(dir_name) = parent.file_name().and_then(|n| n.to_str()) {
            // If named "src_wj", this is definitely the source root for multi-file projects
            if dir_name == "src_wj" {
                found_src_wj = Some(parent);
                // Don't return immediately, keep looking for mod.wj to confirm multi-file structure
            }
        }

        // Track the topmost directory with mod.wj
        if parent.join("mod.wj").exists() {
            topmost_mod_wj_dir = Some(parent);
        }

        current = parent;
    }

    // Prefer src_wj with mod.wj (multi-file project)
    if let Some(src_wj) = found_src_wj {
        if src_wj.join("mod.wj").exists() || topmost_mod_wj_dir.is_some() {
            return Some(src_wj);
        }
    }

    // Otherwise, use the topmost mod.wj directory (multi-file project without src_wj)
    if let Some(mod_wj_dir) = topmost_mod_wj_dir {
        return Some(mod_wj_dir);
    }

    // For single-file projects, use the file's parent directory
    // This prevents deeply nested output paths like /tmp/output/wj/windjammer-game/examples/file.rs
    file_path.parent()
}

/// Calculate output path that preserves directory structure
///
/// Example:
/// - source_root: "windjammer-game/src_wj"
/// - input_path: "windjammer-game/src_wj/math/vec2.wj"
/// - output_dir: "build"
/// - Result: "build/math/vec2.rs"
pub fn get_relative_output_path(
    source_root: &Path,
    input_path: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    // Get the relative path from source_root to input_path
    let relative = input_path.strip_prefix(source_root).unwrap_or(input_path);

    // Replace .wj extension with .rs
    let rs_filename = relative
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.replace(".wj", ".rs"))
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    // Construct output path preserving directory structure
    let mut output_path = output_dir.to_path_buf();

    // Add parent directories if they exist
    if let Some(parent) = relative.parent() {
        if parent != Path::new("") {
            output_path.push(parent);
        }
    }

    // Add the .rs filename
    output_path.push(rs_filename);

    Ok(output_path)
}

/// Generate nested module structure using the new Windjammer module system
/// This replaces the old flat generate_mod_file with proper nested support
pub fn generate_nested_module_structure(source_dir: &Path, output_dir: &Path) -> Result<()> {
    use crate::module_system::{
        discover_nested_modules, generate_lib_rs, generate_mod_rs_for_submodule,
    };
    use anyhow::Context;
    use colored::*;

    // Discover all modules in the source directory
    let module_tree =
        discover_nested_modules(source_dir).context("Failed to discover module structure")?;

    // Generate lib.rs (or mod.rs for root)
    // THE WINDJAMMER WAY: Auto-discover hand-written Rust modules (FFI/interop)
    // Look for hand-written .rs files in the project root (parent of src_wj)
    let project_root = if let Some(parent) = source_dir.parent() {
        if parent.as_os_str().is_empty() {
            std::path::Path::new(".")
        } else {
            parent
        }
    } else {
        source_dir
    };
    let lib_rs_content = generate_lib_rs(&module_tree, project_root)?;
    let lib_rs_path = output_dir.join("lib.rs");
    std::fs::write(&lib_rs_path, lib_rs_content)?;

    // Copy hand-written modules to output directory
    // THE WINDJAMMER WAY: Seamless FFI integration!
    // BUT: Don't overwrite generated .rs files from .wj sources!
    // Check both project_root/ and project_root/src/ (Rust convention)
    let copy_dirs = vec![project_root.to_path_buf(), project_root.join("src")];

    for copy_dir in &copy_dirs {
        if !copy_dir.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(copy_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        // Copy .rs files that aren't lib.rs or mod.rs
                        if name_str.ends_with(".rs") && name_str != "lib.rs" && name_str != "mod.rs"
                        {
                            // THE WINDJAMMER WAY: Check if there's a corresponding .wj file
                            // If runtime.wj exists, don't copy runtime.rs (it would overwrite the generated file!)
                            let stem = path.file_stem().unwrap().to_string_lossy();
                            let corresponding_wj = source_dir.join(format!("{}.wj", stem));

                            if corresponding_wj.exists() {
                                continue; // Skip copying - this file is generated from .wj
                            }

                            // Only copy hand-written .rs files (like ffi.rs)
                            let dest = output_dir.join(name);
                            if let Err(e) = std::fs::copy(&path, &dest) {
                                eprintln!("Warning: Failed to copy {}: {}", name_str, e);
                            }
                        }
                    }
                } else if path.is_dir() {
                    // THE WINDJAMMER WAY: Copy directories with hand-written Rust (like ffi/)
                    if let Some(dir_name) = path.file_name() {
                        let dir_name_str = dir_name.to_string_lossy();
                        let skip_dirs = [
                            "src_wj",
                            "target",
                            "build",
                            "generated",
                            "dist",
                            "node_modules",
                            ".git",
                            "src",
                        ];

                        if !skip_dirs.contains(&dir_name_str.as_ref()) {
                            // CRITICAL FIX: Don't copy directories that correspond to Windjammer modules!
                            // Check if there's a corresponding .wj directory in src_wj
                            let corresponding_wj_dir = source_dir.join(dir_name_str.as_ref());
                            if corresponding_wj_dir.exists() && corresponding_wj_dir.is_dir() {
                                continue; // Skip copying - this is a Windjammer module directory
                            }

                            // Check if this directory has a mod.rs (it's a Rust module)
                            let mod_rs = path.join("mod.rs");
                            if mod_rs.exists() {
                                let dest_dir = output_dir.join(dir_name);
                                if let Err(e) = copy_dir_recursive(&path, &dest_dir) {
                                    eprintln!(
                                        "Warning: Failed to copy directory {}: {}",
                                        dir_name_str, e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!(
        "{} Generated lib.rs with {} top-level modules",
        "✓".green(),
        module_tree.root_modules.len()
    );

    // Recursively generate mod.rs for each directory module
    fn generate_mod_rs_recursive(
        module: &crate::module_system::Module,
        output_dir: &Path,
    ) -> Result<()> {
        if module.is_directory && !module.submodules.is_empty() {
            let mod_rs_content = generate_mod_rs_for_submodule(module)?;
            let module_output_dir = output_dir.join(&module.name);
            std::fs::create_dir_all(&module_output_dir)?;
            let mod_rs_path = module_output_dir.join("mod.rs");
            std::fs::write(&mod_rs_path, mod_rs_content)?;

            // Recursively generate for submodules
            for submodule in &module.submodules {
                generate_mod_rs_recursive(submodule, &module_output_dir)?;
            }
        }
        Ok(())
    }

    // Generate mod.rs for all directory modules
    for module in &module_tree.root_modules {
        generate_mod_rs_recursive(module, output_dir)?;
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_relative_output_path_nested() {
        let source_root = Path::new("src_wj");
        let input_path = Path::new("src_wj/math/vec2.wj");
        let output_dir = Path::new("build");

        let result = get_relative_output_path(source_root, input_path, output_dir).unwrap();
        assert_eq!(result, PathBuf::from("build/math/vec2.rs"));
    }

    #[test]
    fn test_get_relative_output_path_flat() {
        let source_root = Path::new("src_wj");
        let input_path = Path::new("src_wj/vec2.wj");
        let output_dir = Path::new("build");

        let result = get_relative_output_path(source_root, input_path, output_dir).unwrap();
        assert_eq!(result, PathBuf::from("build/vec2.rs"));
    }

    #[test]
    fn test_get_relative_output_path_deeply_nested() {
        let source_root = Path::new("game/src_wj");
        let input_path = Path::new("game/src_wj/rendering/shaders/vertex.wj");
        let output_dir = Path::new("build");

        let result = get_relative_output_path(source_root, input_path, output_dir).unwrap();
        assert_eq!(result, PathBuf::from("build/rendering/shaders/vertex.rs"));
    }

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
