pub mod lexer;
pub mod parser;
pub mod analyzer;
pub mod codegen;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::collections::HashMap;
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
    
    // Process each file
    for file_path in &wj_files {
        print!("  {} {:?}... ", "Compiling".cyan(), file_path.file_name().unwrap());
        
        match compile_file(file_path, output, target) {
            Ok(_) => println!("{}", "✓".green()),
            Err(e) => {
                println!("{}", "✗".red());
                eprintln!("    {}: {}", "Error".red().bold(), e);
                has_errors = true;
            }
        }
    }
    
    if !has_errors {
        // Create Cargo.toml if it doesn't exist
        create_cargo_toml(output)?;
        
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
        }
    }
    
    /// Compile a module and all its dependencies
    fn compile_module(&mut self, module_path: &str) -> Result<()> {
        // Skip if already compiled
        if self.compiled_modules.contains_key(module_path) {
            return Ok(());
        }
        
        // Find the module file
        let file_path = self.resolve_module_path(module_path)?;
        
        // Read and parse
        let source = std::fs::read_to_string(&file_path)?;
        let mut lexer = lexer::Lexer::new(&source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let program = parser.parse()
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", module_path, e))?;
        
        // Find dependencies (use statements) and compile them first
        for item in &program.items {
            if let parser::Item::Use(path) = item {
                let dep_path = path.join(".");
                // Only recursively compile std modules for now
                if dep_path.starts_with("std.") {
                    self.compile_module(&dep_path)?;
                }
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
        let module_name = module_path.split('.').last().unwrap();
        let wrapped_code = format!("pub mod {} {{\n{}\n}}\n", module_name, rust_code);
        
        // Store compiled module
        self.compiled_modules.insert(module_path.to_string(), wrapped_code);
        
        Ok(())
    }
    
    /// Resolve module path to file path
    fn resolve_module_path(&self, module_path: &str) -> Result<PathBuf> {
        if module_path.starts_with("std.") {
            // stdlib module: std.json -> $STDLIB/json.wj
            let module_name = module_path.strip_prefix("std.").unwrap();
            let file_path = self.stdlib_path.join(format!("{}.wj", module_name));
            if file_path.exists() {
                Ok(file_path)
            } else {
                Err(anyhow::anyhow!("Stdlib module not found: {} (looked in {:?})", module_path, file_path))
            }
        } else {
            // TODO: Relative imports for user modules
            Err(anyhow::anyhow!("Only std.* imports are supported currently"))
        }
    }
    
    /// Get all compiled modules in dependency order
    fn get_compiled_modules(&self) -> Vec<String> {
        // For now, just return modules in any order
        // TODO: Topological sort for proper dependency order
        self.compiled_modules.values().cloned().collect()
    }
}

fn compile_file(input_path: &PathBuf, output_dir: &PathBuf, target: CompilationTarget) -> Result<()> {
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
        if let parser::Item::Use(path) = item {
            let module_path = path.join(".");
            if module_path.starts_with("std.") {
                module_compiler.compile_module(&module_path)?;
            }
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
    
    Ok(())
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

fn create_cargo_toml(output_dir: &PathBuf) -> Result<()> {
    let cargo_toml_path = output_dir.join("Cargo.toml");
    
    if !cargo_toml_path.exists() {
        let cargo_toml_content = r#"[package]
name = "windjammer-output"
version = "0.1.0"
edition = "2021"

[dependencies]
# Add common dependencies that Windjammer examples might use
tokio = { version = "1", features = ["full"] }
axum = "0.7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
anyhow = "1"
rayon = "1"
wasm-bindgen = "0.2"
web-sys = "0.3"
js-sys = "0.3"
"#;
        
        std::fs::write(cargo_toml_path, cargo_toml_content)?;
    }
    
    Ok(())
}

