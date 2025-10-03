pub mod lexer;
pub mod parser;
pub mod analyzer;
pub mod codegen;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;

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
        Commands::Build { path, output } => {
            build_project(&path, &output)?;
        }
        Commands::Check { path } => {
            check_project(&path)?;
        }
    }
    
    Ok(())
}

fn build_project(path: &PathBuf, output: &PathBuf) -> Result<()> {
    use colored::*;
    
    println!("{} Windjammer files in: {:?}", "Building".green().bold(), path);
    
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
        
        match compile_file(file_path, output) {
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

fn compile_file(input_path: &PathBuf, output_dir: &PathBuf) -> Result<()> {
    // Read source file
    let source = std::fs::read_to_string(input_path)?;
    
    // Lex
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    
    // Parse
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    
    // Analyze
    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, registry) = analyzer.analyze_program(&program)
        .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;
    
    // Generate Rust code
    let mut codegen = codegen::CodeGenerator::new(registry);
    let rust_code = codegen.generate_program(&program, &analyzed);
    
    // Write output file
    let output_filename = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
    
    let output_path = output_dir.join(format!("{}.rs", output_filename));
    std::fs::write(output_path, rust_code)?;
    
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

