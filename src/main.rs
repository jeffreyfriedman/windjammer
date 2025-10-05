pub mod lexer;
pub mod parser;
pub mod analyzer;
pub mod codegen;
pub mod source_map;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::collections::{HashMap, HashSet};
use anyhow::Result;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CompilationTarget {
    /// WebAssembly (default)
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
#[command(about = "A Go-like language that transpiles to Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build .wj files and transpile to Rust
    Build {
        /// Directory containing .wj files (defaults to current directory)
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        
        /// Output directory for generated Rust files
        #[arg(short, long, default_value = "output")]
        output: PathBuf,
        
        /// Compilation target (wasm, node, python, c)
        #[arg(short, long, default_value = "wasm")]
        target: CompilationTarget,
    },
    
    /// Check .wj files for errors without generating code
    Check {
        /// Directory containing .wj files (defaults to current directory)
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Build { path, output, target } => {
            build_project(&path, &output, target)?;
        }
        Commands::Check { path } => {
            check_project(&path)?;
        }
    }
    
    Ok(())
}

fn build_project(path: &PathBuf, output: &PathBuf, target: CompilationTarget) -> Result<()> {
    use colored::*;
    
    println!("{} Windjammer files in: {:?}", "Building".green().bold(), path);
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
    
    // Process each file
    for file_path in &wj_files {
        print!("  {} {:?}... ", "Compiling".cyan(), file_path.file_name().unwrap());
        
        match compile_file(file_path, output, target) {
            Ok(imported_modules) => {
                println!("{}", "✓".green());
                // Collect imported stdlib modules
                all_stdlib_modules.extend(imported_modules);
            }
            Err(e) => {
                println!("{}", "✗".red());
                eprintln!("    {}: {}", "Error".red().bold(), e);
                has_errors = true;
            }
        }
    }
    
    if !has_errors {
        // Create Cargo.toml with dependencies for imported stdlib modules
        create_cargo_toml_with_deps(output, &all_stdlib_modules)?;
        
        println!("\n{} Transpilation complete!", "Success!".green().bold());
        println!("Output directory: {:?}", output);
        println!("\nTo run the generated code:");
        println!("  cd {:?}", output);
        println!("  cargo run");
    } else {
        println!("\n{} Compilation failed with errors", "Error:".red().bold());
    }
    
    Ok(())
}

fn check_project(path: &PathBuf) -> Result<()> {
    use colored::*;
    
    println!("{} Windjammer files in: {:?}", "Checking".green().bold(), path);
    
    let wj_files = find_wj_files(path)?;
    
    if wj_files.is_empty() {
        println!("{} No .wj files found", "Warning:".yellow().bold());
        return Ok(());
    }
    
    println!("Found {} file(s)", wj_files.len());
    
    let mut has_errors = false;
    
    for file_path in &wj_files {
        print!("  {} {:?}... ", "Checking".cyan(), file_path.file_name().unwrap());
        
        match check_file(file_path) {
            Ok(_) => println!("{}", "✓".green()),
            Err(e) => {
                println!("{}", "✗".red());
                eprintln!("    {}: {}", "Error".red().bold(), e);
                has_errors = true;
            }
        }
    }
    
    if !has_errors {
        println!("\n{} All files are valid", "Success!".green().bold());
    } else {
        println!("\n{} Check failed with errors", "Error:".red().bold());
    }
    
    Ok(())
}

fn find_wj_files(path: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut wj_files = Vec::new();
    
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("wj") {
            wj_files.push(path.clone());
        }
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            
            if entry_path.is_file() {
                if entry_path.extension().and_then(|s| s.to_str()) == Some("wj") {
                    wj_files.push(entry_path);
                }
            }
        }
    }
    
    Ok(wj_files)
}

/// Tracks compiled modules and their dependencies
struct ModuleCompiler {
    compiled_modules: HashMap<String, String>, // module path -> generated Rust code
    target: CompilationTarget,
    stdlib_path: PathBuf,
    imported_stdlib_modules: HashSet<String>, // Track which stdlib modules are used
}

impl ModuleCompiler {
    fn new(target: CompilationTarget) -> Self {
        // Find stdlib directory
        // Check WINDJAMMER_STDLIB env var first, then fallback to ./std
        let stdlib_path = std::env::var("WINDJAMMER_STDLIB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("std"));
        
        Self {
            compiled_modules: HashMap::new(),
            target,
            stdlib_path,
            imported_stdlib_modules: HashSet::new(),
        }
    }
    
    /// Compile a module and all its dependencies
    /// source_file: The file that is importing this module (for relative path resolution)
    fn compile_module(&mut self, module_path: &str, source_file: Option<&PathBuf>) -> Result<()> {
        // Skip if already compiled
        if self.compiled_modules.contains_key(module_path) {
            return Ok(());
        }
        
        // Find the module file
        let file_path = self.resolve_module_path(module_path, source_file)?;
        
        // Read and parse
        let source = std::fs::read_to_string(&file_path)?;
        let mut lexer = lexer::Lexer::new(&source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let program = parser.parse()
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", module_path, e))?;
        
        // Find dependencies (use statements) and compile them first
        for item in &program.items {
            if let parser::Item::Use { path, alias: _ } = item {
                let dep_path = path.join(".");
                // Recursively compile both std modules and relative imports
                self.compile_module(&dep_path, Some(&file_path))?;
            }
        }
        
        // Analyze and generate code
        let mut analyzer = analyzer::Analyzer::new();
        let (analyzed, registry) = analyzer.analyze_program(&program)
            .map_err(|e| anyhow::anyhow!("Analysis error in {}: {}", module_path, e))?;
        
        let mut codegen = codegen::CodeGenerator::new_for_module(registry, self.target);
        let rust_code = codegen.generate_program(&program, &analyzed);
        
        // Wrap module code in a Rust mod block
        // e.g., std.json becomes: pub mod json { ... }
        // For relative imports like ./utils or ../utils/helpers, extract module name
        let module_name = if module_path.starts_with("./") || module_path.starts_with("../") {
            // Extract final component: ./utils -> utils, ../utils/helpers -> helpers
            let stripped = module_path.strip_prefix("./")
                .or_else(|| module_path.strip_prefix("../"))
                .unwrap_or(module_path);
            stripped.split('/').last().unwrap_or(stripped)
        } else if module_path.contains('.') {
            // std.json -> json
            module_path.split('.').last().unwrap()
        } else {
            module_path
        };
        let wrapped_code = format!("pub mod {} {{\n{}\n}}\n", module_name, rust_code);
        
        // Store compiled module
        self.compiled_modules.insert(module_path.to_string(), wrapped_code);
        
        // Track stdlib module for dependency generation
        if module_path.starts_with("std.") {
            self.imported_stdlib_modules.insert(module_path.to_string());
        }
        
        Ok(())
    }
    
    /// Resolve module path to file path
    /// source_file: The file that is importing this module (for relative path resolution)
    fn resolve_module_path(&self, module_path: &str, source_file: Option<&PathBuf>) -> Result<PathBuf> {
        if module_path.starts_with("std.") {
            // stdlib module: std.json -> $STDLIB/json.wj
            let module_name = module_path.strip_prefix("std.").unwrap();
            let file_path = self.stdlib_path.join(format!("{}.wj", module_name));
            if file_path.exists() {
                Ok(file_path)
            } else {
                Err(anyhow::anyhow!("Stdlib module not found: {} (looked in {:?})", module_path, file_path))
            }
        } else if module_path.starts_with("./") || module_path.starts_with("../") {
            // Relative import: ./module or ../module
            let source_dir = source_file
                .and_then(|f| f.parent())
                .ok_or_else(|| anyhow::anyhow!("Cannot resolve relative import without source file"))?;
            
            // module_path is already in form "./utils" or "./utils/helpers"
            // Just strip the leading ./ or ../
            let relative_part = if module_path.starts_with("./") {
                module_path.strip_prefix("./").unwrap()
            } else {
                module_path.strip_prefix("../").unwrap()
            };
            
            // Resolve relative path from source directory
            let base_path = if module_path.starts_with("../") {
                source_dir.parent().ok_or_else(|| anyhow::anyhow!("Cannot go up from root directory"))?
            } else {
                source_dir
            };
            
            let resolved = base_path.join(relative_part).with_extension("wj");
            
            // Try file directly first
            if resolved.exists() {
                return Ok(resolved);
            }
            
            // Try as directory module: ./utils -> utils/mod.wj
            let mod_path = base_path.join(relative_part).join("mod.wj");
            if mod_path.exists() {
                return Ok(mod_path);
            }
            
            Err(anyhow::anyhow!(
                "User module not found: {} (looked in {:?} and {:?})",
                module_path, resolved, mod_path
            ))
        } else {
            // Absolute import (future: github.com/user/repo style)
            Err(anyhow::anyhow!("Absolute imports not yet supported: {}", module_path))
        }
    }
    
    /// Get all compiled modules in dependency order
    fn get_compiled_modules(&self) -> Vec<String> {
        // For now, just return modules in any order
        // TODO: Topological sort for proper dependency order
        self.compiled_modules.values().cloned().collect()
    }
    
    /// Get Cargo dependencies for imported stdlib modules
    fn get_cargo_dependencies(&self) -> Vec<String> {
        let mut deps = Vec::new();
        
        for module in &self.imported_stdlib_modules {
            let module_deps = match module.as_str() {
                "std.json" => vec![
                    "serde = \"1.0\"",
                    "serde_json = \"1.0\"",
                ],
                "std.csv" => vec![
                    "csv = \"1.3\"",
                ],
                "std.http" => vec![
                    "reqwest = { version = \"0.11\", features = [\"blocking\"] }",
                ],
                "std.time" => vec![
                    "chrono = \"0.4\"",
                ],
                "std.log" => vec![
                    "log = \"0.4\"",
                    "env_logger = \"0.11\"",
                ],
                "std.regex" => vec![
                    "regex = \"1.10\"",
                ],
                "std.encoding" => vec![
                    "base64 = \"0.21\"",
                    "hex = \"0.4\"",
                    "urlencoding = \"2.1\"",
                ],
                "std.crypto" => vec![
                    "sha2 = \"0.10\"",
                    "md5 = \"0.7\"",
                ],
                // std.fs, std.strings, std.math don't need external dependencies (use std)
                _ => vec![],
            };
            
            for dep in module_deps {
                if !deps.contains(&dep.to_string()) {
                    deps.push(dep.to_string());
                }
            }
        }
        
        deps
    }
}

fn compile_file(input_path: &PathBuf, output_dir: &PathBuf, target: CompilationTarget) -> Result<HashSet<String>> {
    let mut module_compiler = ModuleCompiler::new(target);
    
    // Read source file
    let source = std::fs::read_to_string(input_path)?;
    
    // Lex
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    
    // Parse
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    
    // Compile dependencies first
    for item in &program.items {
        if let parser::Item::Use { path, alias: _ } = item {
            let module_path = path.join(".");
            // Compile both std.* and relative imports (./ or ../)
            module_compiler.compile_module(&module_path, Some(input_path))?;
        }
    }
    
    // Analyze main file
    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, registry) = analyzer.analyze_program(&program)
        .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;
    
    // Generate Rust code for main file
    let mut codegen = codegen::CodeGenerator::new(registry, target);
    let main_code = codegen.generate_program(&program, &analyzed);
    
    // Combine modules: dependencies first, then main code
    let mut final_code = String::new();
    
    // Add compiled dependencies
    for module_code in module_compiler.get_compiled_modules() {
        final_code.push_str(&module_code);
        final_code.push_str("\n\n");
    }
    
    // Add main code
    final_code.push_str(&main_code);
    
    // Write output file
    let output_filename = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
    
    let output_path = output_dir.join(format!("{}.rs", output_filename));
    std::fs::write(output_path, final_code)?;
    
    // Return imported stdlib modules
    Ok(module_compiler.imported_stdlib_modules)
}

fn check_file(file_path: &PathBuf) -> Result<()> {
    let source = std::fs::read_to_string(file_path)?;
    
    // Lex
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    
    // Parse
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    
    // Analyze
    let mut analyzer = analyzer::Analyzer::new();
    let (_analyzed, _registry) = analyzer.analyze_program(&program)
        .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;
    
    Ok(())
}

fn create_cargo_toml_with_deps(output_dir: &PathBuf, imported_modules: &HashSet<String>) -> Result<()> {
    let cargo_toml_path = output_dir.join("Cargo.toml");
    
    // Get dependencies based on imported modules
    let mut deps = Vec::new();
    
    for module in imported_modules {
        let module_deps = match module.as_str() {
            "std.json" => vec![
                "serde = { version = \"1.0\", features = [\"derive\"] }",
                "serde_json = \"1.0\"",
            ],
            "std.csv" => vec![
                "csv = \"1.3\"",
            ],
            "std.http" => vec![
                "reqwest = { version = \"0.11\", features = [\"blocking\"] }",
            ],
            "std.time" => vec![
                "chrono = \"0.4\"",
            ],
            "std.log" => vec![
                "log = \"0.4\"",
                "env_logger = \"0.11\"",
            ],
            "std.regex" => vec![
                "regex = \"1.10\"",
            ],
            "std.encoding" => vec![
                "base64 = \"0.21\"",
                "hex = \"0.4\"",
                "urlencoding = \"2.1\"",
            ],
            "std.crypto" => vec![
                "sha2 = \"0.10\"",
                "md5 = \"0.7\"",
                "rand = \"0.8\"",
            ],
            _ => vec![],
        };
        
        for dep in module_deps {
            if !deps.contains(&dep.to_string()) {
                deps.push(dep.to_string());
            }
        }
    }
    
    // Always include WASM dependencies for @export support
    if !deps.iter().any(|d| d.starts_with("wasm-bindgen")) {
        deps.push("wasm-bindgen = \"0.2\"".to_string());
    }
    if !deps.iter().any(|d| d.starts_with("web-sys")) {
        deps.push("web-sys = \"0.3\"".to_string());
    }
    if !deps.iter().any(|d| d.starts_with("js-sys")) {
        deps.push("js-sys = \"0.3\"".to_string());
    }
    
    // Build Cargo.toml content
    let mut cargo_toml_content = String::from(
        "[package]\nname = \"windjammer-output\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n"
    );
    
    for dep in &deps {
        cargo_toml_content.push_str(dep);
        cargo_toml_content.push('\n');
    }
    
    // Add [[bin]] section if main.rs exists
    let main_rs = output_dir.join("main.rs");
    if main_rs.exists() {
        cargo_toml_content.push_str("\n[[bin]]\nname = \"windjammer-output\"\npath = \"main.rs\"\n");
    }
    
    std::fs::write(cargo_toml_path, cargo_toml_content)?;
    
    Ok(())
}

