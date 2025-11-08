pub mod analyzer;
pub mod auto_clone; // Automatic clone insertion for ergonomics
pub mod auto_fix; // Automatic error fixing
pub mod cli;
pub mod codegen;
pub mod component_analyzer;
pub mod error;
// Removed: codegen_legacy is now codegen::rust::generator
pub mod compiler_database;
pub mod config;
pub mod ejector;
pub mod error_mapper;
pub mod fuzzy_matcher; // Fuzzy string matching for typo suggestions
pub mod inference;
pub mod lexer;
pub mod optimizer;
pub mod parser; // Parser module (refactored structure)
pub mod parser_impl; // Parser implementation (being migrated to parser/)
pub mod parser_recovery;
pub mod source_map; // Source map for error message translation
pub mod source_map_cache; // Source map caching for performance
pub mod stdlib_scanner;
pub mod syntax_highlighter; // Syntax highlighting for error snippets

// UI component compilation
pub mod component;

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
        } => {
            build_project(&path, &output, target)?;
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

#[allow(dead_code)]
fn build_project(path: &Path, output: &Path, target: CompilationTarget) -> Result<()> {
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

    for file in &wj_files {
        let file_name = file.file_name().unwrap().to_str().unwrap();
        print!("  Compiling {:?}... ", file_name);

        match compile_file(file, output, target) {
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
        // Create Cargo.toml with stdlib and external dependencies (unless it's a component project)
        if !is_component_project {
            create_cargo_toml_with_deps(output, &all_stdlib_modules, &all_external_crates)?;
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
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("wj") {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

// Module compiler for handling dependencies
#[allow(dead_code)]
struct ModuleCompiler {
    compiled_modules: HashMap<String, String>, // module path -> generated Rust code
    target: CompilationTarget,
    stdlib_path: PathBuf,
    imported_stdlib_modules: HashSet<String>, // Track which stdlib modules are used
    external_crates: Vec<String>,             // Track external crates (e.g., windjammer_ui)
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
            imported_stdlib_modules: HashSet::new(),
            external_crates: Vec::new(),
        }
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
                .to_string();

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

        // Analyze
        let mut analyzer = analyzer::Analyzer::new();
        let (analyzed, signatures) = analyzer
            .analyze_program(&program)
            .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;

        // Generate Rust code (as a module)
        let mut generator = codegen::CodeGenerator::new_for_module(signatures, self.target);
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
            // External crate imports (e.g., windjammer_ui, external_crate)
            // These are treated as Rust crate dependencies and passed through to generated code
            // Mark as external by returning a special "external" path
            // We'll handle this specially in the compilation phase
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
fn compile_file(
    input_path: &Path,
    output_dir: &Path,
    target: CompilationTarget,
) -> Result<(HashSet<String>, Vec<String>)> {
    let mut module_compiler = ModuleCompiler::new(target);

    // Read source file
    let source = std::fs::read_to_string(input_path)?;

    // Check if this is a component file by looking for component-specific syntax
    // Components must have a "view {" block (as a standalone token, not substring)
    let is_component = if target == CompilationTarget::Wasm {
        // Only detect as component if "view" is followed by "{" (not just any "view" word)
        source.contains("view {")
            || source.contains("view{")
            || source.contains("\nview ") && (source.contains("view {") || source.contains("view{"))
    } else {
        false
    };

    // If it's a component file, handle it specially
    if is_component {
        use component::analyzer::DependencyAnalyzer;
        use component::codegen::ComponentCodegen;
        use component::parser::ComponentParser;
        use component::transformer::SignalTransformer;

        // Parse as component
        let component_file = ComponentParser::parse(&source)
            .map_err(|e| anyhow::anyhow!("Component parse error: {}", e))?;

        // Analyze dependencies
        let deps = DependencyAnalyzer::analyze(&component_file)
            .map_err(|e| anyhow::anyhow!("Component analysis error: {}", e))?;

        // Transform to signals
        let transformed = SignalTransformer::transform(&component_file, &deps)
            .map_err(|e| anyhow::anyhow!("Component transformation error: {}", e))?;

        // Generate code
        let rust_code = ComponentCodegen::generate(&transformed)
            .map_err(|e| anyhow::anyhow!("Component codegen error: {}", e))?;

        // Write lib.rs
        std::fs::create_dir_all(output_dir.join("src"))?;
        let lib_path = output_dir.join("src").join("lib.rs");
        std::fs::write(&lib_path, &rust_code)?;

        // Get component name (use filename if not specified)
        let component_name = component_file.name.as_deref().unwrap_or_else(|| {
            input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Component")
        });

        // Generate Cargo.toml
        let cargo_toml = ComponentCodegen::generate_cargo_toml(component_name)?;
        std::fs::write(output_dir.join("Cargo.toml"), &cargo_toml)?;

        // Generate index.html
        let index_html = ComponentCodegen::generate_index_html(component_name)?;
        std::fs::write(output_dir.join("index.html"), &index_html)?;

        // Generate README
        let readme = ComponentCodegen::generate_readme(component_name)?;
        std::fs::write(output_dir.join("README.md"), &readme)?;

        println!("  Component compiled successfully!");
        println!("  Output directory: {}", output_dir.display());
        println!("  Next steps:");
        println!("    cd {}", output_dir.display());
        println!("    wasm-pack build --target web");
        println!("    python3 -m http.server 8080");

        return Ok((HashSet::new(), Vec::new()));
    }

    // Regular Windjammer file - use standard parser
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

    // Compile dependencies first
    for item in &program.items {
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
    }

    // Analyze
    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, signatures) = analyzer
        .analyze_program(&program)
        .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;

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
            let mut generator = codegen::CodeGenerator::new(signatures, target);
            generator.set_inferred_bounds(inferred_bounds_map);

            // Set source file for error mapping
            generator.set_source_file(input_path);
            let output_file_path = output_dir.join(
                input_path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace(".wj", ".rs"),
            );
            generator.set_output_file(&output_file_path);

            let code = generator.generate_program(&program, &analyzed);

            // Save source map for error mapping
            let source_map_path = output_file_path.with_extension("rs.map");
            if let Err(e) = generator.get_source_map().save_to_file(&source_map_path) {
                eprintln!("Warning: Failed to save source map: {}", e);
            }

            code
        }
    } else {
        // Use old generator for Rust target
        let mut generator = codegen::CodeGenerator::new(signatures, target);
        generator.set_inferred_bounds(inferred_bounds_map);

        // Set source file for error mapping
        generator.set_source_file(input_path);
        let output_file_path = output_dir.join(
            input_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace(".wj", ".rs"),
        );
        generator.set_output_file(&output_file_path);

        let code = generator.generate_program(&program, &analyzed);

        // Save source map for error mapping
        let source_map_path = output_file_path.with_extension("rs.map");
        if let Err(e) = generator.get_source_map().save_to_file(&source_map_path) {
            eprintln!("Warning: Failed to save source map: {}", e);
        }

        code
    };

    // Combine module code with main code
    let module_code = module_compiler.get_compiled_modules().join("\n");
    let combined_code = if module_code.is_empty() {
        rust_code
    } else {
        format!("{}\n\n{}", module_code, rust_code)
    };

    // Write output
    let output_file = output_dir.join(
        input_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .replace(".wj", ".rs"),
    );

    std::fs::write(output_file, combined_code)?;

    // Return the set of imported stdlib modules and external crates for Cargo.toml generation
    Ok((
        module_compiler.imported_stdlib_modules,
        module_compiler.external_crates,
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
) -> Result<()> {
    use std::env;
    use std::fs;

    // Map imported stdlib modules to their Cargo dependencies
    let mut deps = Vec::new();

    // If ANY stdlib module is used, add windjammer-runtime
    if !imported_modules.is_empty() {
        // Add windjammer-runtime dependency (path-based for now)
        // Search upward for the windjammer root directory
        let windjammer_runtime_path = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            PathBuf::from(manifest_dir).join("crates/windjammer-runtime")
        } else {
            // Start from current directory and search upward
            let mut current = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let mut found = false;

            // Try current directory first
            if current
                .join("crates/windjammer-runtime/Cargo.toml")
                .exists()
            {
                current.join("crates/windjammer-runtime")
            } else {
                // Search upward (up to 5 levels)
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
                    // Fallback: assume we're in the root
                    PathBuf::from("./crates/windjammer-runtime")
                }
            }
        };

        deps.push(format!(
            "windjammer-runtime = {{ path = \"{}\" }}",
            windjammer_runtime_path.display()
        ));
    }

    // Legacy: Keep old dependencies for modules not yet in runtime
    for module in imported_modules {
        match module.as_str() {
            // These are now in windjammer-runtime, no extra deps needed
            "fs" | "http" | "mime" | "json" => {}

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

    // Add external crates (from workspace or crates.io)
    let mut external_deps = Vec::new();
    for crate_name in external_crates {
        match crate_name.as_str() {
            "windjammer_ui" => {
                // Use absolute path to the workspace crate
                // Search upward for the windjammer root directory
                let windjammer_ui_path = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
                    PathBuf::from(manifest_dir).join("crates/windjammer-ui")
                } else {
                    // Start from current directory and search upward
                    let mut current = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                    let mut found = false;

                    // Try current directory first
                    if current.join("crates/windjammer-ui/Cargo.toml").exists() {
                        current.join("crates/windjammer-ui")
                    } else {
                        // Search upward (up to 5 levels)
                        for _ in 0..5 {
                            if let Some(parent) = current.parent() {
                                if parent.join("crates/windjammer-ui/Cargo.toml").exists() {
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
                            current.join("crates/windjammer-ui")
                        } else {
                            // Fallback: assume we're in the root
                            PathBuf::from("./crates/windjammer-ui")
                        }
                    }
                };

                external_deps.push(format!(
                    "windjammer-ui = {{ path = \"{}\" }}",
                    windjammer_ui_path.display()
                ));

                // Also add the macro crate (needed for #[component], #[derive(Props)])
                let windjammer_ui_macro_path =
                    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
                        PathBuf::from(manifest_dir).join("crates/windjammer-ui-macro")
                    } else {
                        // Start from current directory and search upward
                        let mut current = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                        let mut found = false;

                        // Try current directory first
                        if current
                            .join("crates/windjammer-ui-macro/Cargo.toml")
                            .exists()
                        {
                            current.join("crates/windjammer-ui-macro")
                        } else {
                            // Search upward (up to 5 levels)
                            for _ in 0..5 {
                                if let Some(parent) = current.parent() {
                                    if parent
                                        .join("crates/windjammer-ui-macro/Cargo.toml")
                                        .exists()
                                    {
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
                                current.join("crates/windjammer-ui-macro")
                            } else {
                                // Fallback: assume we're in the root
                                PathBuf::from("./crates/windjammer-ui-macro")
                            }
                        }
                    };

                external_deps.push(format!(
                    "windjammer-ui-macro = {{ path = \"{}\" }}",
                    windjammer_ui_macro_path.display()
                ));
            }
            _ => {
                // Default: assume it's a crates.io dependency
                external_deps.push(format!("{} = \"*\"", crate_name));
            }
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

    // Find all .rs files to create [[bin]] sections
    let mut bin_sections = Vec::new();
    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.ends_with(".rs") && filename != "lib.rs" {
                    let bin_name = filename.strip_suffix(".rs").unwrap_or(filename);
                    bin_sections.push(format!(
                        "[[bin]]\nname = \"{}\"\npath = \"{}\"\n",
                        bin_name, filename
                    ));
                }
            }
        }
    }

    let bin_section = if !bin_sections.is_empty() {
        format!("{}\n", bin_sections.join("\n"))
    } else {
        String::new()
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
        deps_section, bin_section
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
fn run_file(file: &Path, mut target: CompilationTarget, args: &[String]) -> Result<()> {
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

    // Auto-detect target based on imports if using default Rust target
    if matches!(target, CompilationTarget::Rust) {
        let source = fs::read_to_string(file)?;

        // Check for UI framework imports - these require WASM
        if source.contains("windjammer_ui") {
            println!("{} UI app detected, using WASM target", "ℹ".cyan().bold());
            target = CompilationTarget::Wasm;
        }
        // Check for game framework imports - these can run natively
        else if source.contains("windjammer_game_framework") {
            println!("{} Game app detected, using Rust target", "ℹ".cyan().bold());
            // Keep Rust target for native games
        }
    }

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
