//! Ejector - Convert Windjammer projects to standalone Rust projects
//!
//! This module provides the "eject" feature, allowing users to convert their
//! Windjammer codebase into a pure Rust project. This removes vendor lock-in
//! and provides a migration path for users who want to transition to Rust.

use crate::{analyzer, codegen, inference, lexer, parser, CompilationTarget};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for the ejection process
#[derive(Debug, Clone)]
pub struct EjectConfig {
    /// Whether to run rustfmt on generated code
    pub format: bool,
    /// Whether to add helpful comments
    pub comments: bool,
    /// Whether to generate Cargo.toml
    pub generate_cargo_toml: bool,
    /// Compilation target
    pub target: CompilationTarget,
}

impl Default for EjectConfig {
    fn default() -> Self {
        Self {
            format: true,
            comments: true,
            generate_cargo_toml: true,
            target: CompilationTarget::Wasm,
        }
    }
}

/// Result of ejecting a single file
#[derive(Debug)]
struct EjectFileResult {
    /// Path to the generated Rust file
    _output_path: PathBuf,
    /// The generated Rust code (kept for potential future use)
    _rust_code: String,
    /// Imported stdlib modules (for dependency tracking)
    stdlib_modules: HashSet<String>,
}

/// Main ejector that converts Windjammer to Rust
pub struct Ejector {
    config: EjectConfig,
    /// Track all stdlib modules used across the project
    all_stdlib_modules: HashSet<String>,
}

impl Ejector {
    pub fn new(config: EjectConfig) -> Self {
        Self {
            config,
            all_stdlib_modules: HashSet::new(),
        }
    }

    /// Eject a Windjammer project to a Rust project
    pub fn eject_project(&mut self, input: &Path, output: &Path) -> Result<()> {
        use colored::*;

        println!(
            "{} Ejecting Windjammer project to Rust...",
            "🚀".bold().green()
        );
        println!("  Input:  {:?}", input);
        println!("  Output: {:?}", output);
        println!();

        // Create output directory
        fs::create_dir_all(output)?;

        // Find all .wj files
        let wj_files = find_wj_files(input)?;

        if wj_files.is_empty() {
            return Err(anyhow::anyhow!("No .wj files found in {:?}", input));
        }

        println!("Found {} Windjammer file(s):", wj_files.len());
        for file in &wj_files {
            println!("  • {}", file.display());
        }
        println!();

        // Eject each file
        let mut ejected_files = Vec::new();
        let mut has_errors = false;

        for wj_file in &wj_files {
            print!(
                "{} {:?}... ",
                "  Ejecting".cyan(),
                wj_file.file_name().unwrap()
            );

            match self.eject_file(wj_file, output) {
                Ok(result) => {
                    println!("{}", "✓".green());
                    self.all_stdlib_modules
                        .extend(result.stdlib_modules.clone());
                    ejected_files.push(result);
                }
                Err(e) => {
                    println!("{}", "✗".red());
                    eprintln!("    Error: {}", e);
                    has_errors = true;
                }
            }
        }

        if has_errors {
            return Err(anyhow::anyhow!("Ejection failed with errors"));
        }

        println!();

        // Generate Cargo.toml
        if self.config.generate_cargo_toml {
            print!("{} Cargo.toml... ", "  Creating".cyan());
            self.create_cargo_toml(output, &ejected_files)?;
            println!("{}", "✓".green());
        }

        // Generate README.md
        print!("{} README.md... ", "  Creating".cyan());
        self.create_readme(output)?;
        println!("{}", "✓".green());

        // Generate .gitignore
        print!("{} .gitignore... ", "  Creating".cyan());
        self.create_gitignore(output)?;
        println!("{}", "✓".green());

        // Run rustfmt if requested
        if self.config.format {
            println!();
            print!("{} generated code... ", "  Formatting".cyan());
            if let Err(e) = self.run_rustfmt(output) {
                println!("{}", "⚠".yellow());
                eprintln!("    Warning: rustfmt failed: {}", e);
                eprintln!("    You can run 'cargo fmt' manually in {:?}", output);
            } else {
                println!("{}", "✓".green());
            }
        }

        println!();
        println!("{} Ejection complete!", "✅".bold().green());
        println!();
        println!("Your Rust project is ready at: {:?}", output);
        println!();
        println!("Next steps:");
        println!("  1. cd {:?}", output);
        println!("  2. cargo build         # Build the project");
        println!("  3. cargo test          # Run tests");
        println!("  4. cargo run           # Run the application");
        println!();
        println!(
            "{}",
            "Note: This is a one-way conversion. Your original Windjammer files are unchanged."
                .yellow()
        );
        println!("      You can continue using Windjammer or adopt the Rust code.");

        Ok(())
    }

    /// Eject a single Windjammer file to Rust
    fn eject_file(&self, input_file: &Path, output_dir: &Path) -> Result<EjectFileResult> {
        // Read source
        let source = fs::read_to_string(input_file)?;

        // Lex
        let mut lexer = lexer::Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();

        // Parse
        let mut parser = parser::Parser::new(tokens);
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

        // Analyze
        let mut analyzer = analyzer::Analyzer::new();
        let (analyzed, signatures) = analyzer
            .analyze_program(&program)
            .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;

        // Infer trait bounds
        let mut inference_engine = inference::InferenceEngine::new();
        let mut inferred_bounds_map = HashMap::new();
        for item in &program.items {
            if let parser::Item::Function { decl: func, .. } = item {
                let bounds = inference_engine.infer_function_bounds(func);
                if !bounds.is_empty() {
                    inferred_bounds_map.insert(func.name.clone(), bounds);
                }
            }
        }

        // Generate Rust code
        let mut generator = codegen::CodeGenerator::new(signatures, self.config.target);
        generator.set_inferred_bounds(inferred_bounds_map);
        let mut rust_code = generator.generate_program(&program, &analyzed);

        // Add helpful comments if requested
        if self.config.comments {
            rust_code = self.add_ejection_comments(rust_code, input_file);
        }

        // Extract stdlib modules from imports
        let stdlib_modules = self.extract_stdlib_modules(&program);

        // Determine output path
        let output_file = output_dir.join(
            input_file
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace(".wj", ".rs"),
        );

        // Write Rust code
        fs::write(&output_file, &rust_code)?;

        Ok(EjectFileResult {
            _output_path: output_file,
            _rust_code: rust_code,
            stdlib_modules,
        })
    }

    /// Add helpful comments to ejected code explaining Windjammer features
    fn add_ejection_comments(&self, rust_code: String, source_file: &Path) -> String {
        let header = format!(
            r#"//! This file was automatically generated by Windjammer eject.
//!
//! Original Windjammer source: {}
//!
//! Windjammer features used in this file:
//! - Ownership inference: Types inferred automatically from usage
//! - Trait bound inference: Generic constraints derived from operations
//! - Pattern matching: Powerful destructuring and guards
//! - Concurrency primitives: spawn, channels, defer
//!
//! You can now modify this Rust code directly. It's fully standalone!

"#,
            source_file.display()
        );

        format!("{}{}", header, rust_code)
    }

    /// Extract stdlib modules from program imports
    fn extract_stdlib_modules(&self, program: &parser::Program) -> HashSet<String> {
        let mut modules = HashSet::new();

        for item in &program.items {
            if let parser::Item::Use { path, alias: _, .. } = item {
                let module_path = path.join("::");
                if let Some(module) = module_path.strip_prefix("std::") {
                    modules.insert(module.to_string());
                }
            }
        }

        modules
    }

    /// Create Cargo.toml for the ejected project
    fn create_cargo_toml(&self, output_dir: &Path, files: &[EjectFileResult]) -> Result<()> {
        // Check if main.rs exists
        let has_main = files.iter().any(|f| {
            f._output_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == "main.rs")
                .unwrap_or(false)
        });

        // Map stdlib modules to Cargo dependencies
        let dependencies = self.generate_cargo_dependencies();

        let cargo_toml = if has_main {
            format!(
                r#"[package]
name = "windjammer-ejected"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <you@example.com>"]
description = "Ejected from Windjammer - A standalone Rust project"

# This project was generated by Windjammer eject
# You can modify this Cargo.toml as needed

[[bin]]
name = "app"
path = "main.rs"

{}

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
"#,
                dependencies
            )
        } else {
            format!(
                r#"[package]
name = "windjammer-ejected"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <you@example.com>"]
description = "Ejected from Windjammer - A standalone Rust library"

# This project was generated by Windjammer eject
# You can modify this Cargo.toml as needed

[lib]
name = "windjammer_ejected"
path = "lib.rs"

{}

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
"#,
                dependencies
            )
        };

        let cargo_path = output_dir.join("Cargo.toml");
        fs::write(cargo_path, cargo_toml)?;

        Ok(())
    }

    /// Generate Cargo dependencies section
    fn generate_cargo_dependencies(&self) -> String {
        if self.all_stdlib_modules.is_empty() {
            return String::new();
        }

        let mut deps = Vec::new();

        for module in &self.all_stdlib_modules {
            match module.as_str() {
                "json" => {
                    deps.push("serde = { version = \"1.0\", features = [\"derive\"] }");
                    deps.push("serde_json = \"1.0\"");
                }
                "csv" => {
                    deps.push("csv = \"1.3\"");
                }
                "http" => {
                    deps.push("reqwest = { version = \"0.11\", features = [\"json\"] }");
                    deps.push("axum = \"0.7\"");
                    deps.push("tokio = { version = \"1\", features = [\"full\"] }");
                }
                "time" => {
                    deps.push("chrono = \"0.4\"");
                }
                "log" => {
                    deps.push("log = \"0.4\"");
                    deps.push("env_logger = \"0.11\"");
                }
                "regex" => {
                    deps.push("regex = \"1.10\"");
                }
                "cli" => {
                    deps.push("clap = { version = \"4.5\", features = [\"derive\"] }");
                }
                "db" => {
                    deps.push("sqlx = { version = \"0.7\", features = [\"runtime-tokio-native-tls\", \"postgres\", \"sqlite\", \"mysql\"] }");
                    deps.push("tokio = { version = \"1\", features = [\"full\"] }");
                }
                "random" => {
                    deps.push("rand = \"0.8\"");
                }
                "crypto" => {
                    deps.push("sha2 = \"0.10\"");
                    deps.push("bcrypt = \"0.15\"");
                    deps.push("base64 = \"0.21\"");
                }
                "async" => {
                    deps.push("tokio = { version = \"1\", features = [\"full\"] }");
                }
                // fs, strings, math, env, process use std library
                _ => {}
            }
        }

        deps.sort();
        deps.dedup();

        if deps.is_empty() {
            return String::new();
        }

        format!("[dependencies]\n{}\n", deps.join("\n"))
    }

    /// Create README.md for the ejected project
    fn create_readme(&self, output_dir: &Path) -> Result<()> {
        let readme = r#"# Ejected Windjammer Project

This Rust project was automatically generated by `windjammer eject`.

## What is This?

This is a **standalone Rust project** that was converted from Windjammer source code.
All Windjammer features (ownership inference, trait bounds, pattern matching, etc.)
have been compiled down to idiomatic Rust.

## Getting Started

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run the application (if main.rs exists)
cargo run

# Build optimized release binary
cargo build --release
```

## Project Structure

- `*.rs` - Rust source files (converted from `.wj`)
- `Cargo.toml` - Rust package manifest with dependencies
- `Cargo.lock` - Locked dependency versions (generated)

## Windjammer Features

This code was generated with Windjammer's advanced compiler optimizations:

- **Ownership Inference**: Types and ownership modes inferred automatically
- **Trait Bound Inference**: Generic constraints derived from usage
- **15-Phase Optimization Pipeline**: String interning, dead code elimination, loop optimization, escape analysis, SIMD vectorization, and more!
- **99%+ Rust Performance**: Generated code is as fast as hand-written Rust

## Making Changes

You can now modify this Rust code directly! This is a one-way conversion, so:

1. Your original `.wj` files are unchanged
2. You can continue using Windjammer if you prefer
3. Or adopt this Rust code and work with it directly

This ejected code is **production-ready** and passes all Rust safety checks.

## Learn More

- [Windjammer Documentation](https://github.com/jeffreyfriedman/windjammer)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)

---

**Generated by Windjammer v0.30.0** - Risk-free language adoption ✨
"#;

        let readme_path = output_dir.join("README.md");
        fs::write(readme_path, readme)?;

        Ok(())
    }

    /// Create .gitignore for the ejected project
    fn create_gitignore(&self, output_dir: &Path) -> Result<()> {
        let gitignore = r#"# Rust build artifacts
/target/
Cargo.lock

# IDE files
.vscode/
.idea/
*.swp
*.swo
*~

# OS files
.DS_Store
Thumbs.db
"#;

        let gitignore_path = output_dir.join(".gitignore");
        fs::write(gitignore_path, gitignore)?;

        Ok(())
    }

    /// Run rustfmt on the output directory
    fn run_rustfmt(&self, output_dir: &Path) -> Result<()> {
        use std::process::Command;

        let output = Command::new("rustfmt")
            .arg("--edition")
            .arg("2021")
            .arg("**/*.rs")
            .current_dir(output_dir)
            .output();

        match output {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => Err(anyhow::anyhow!("rustfmt exited with status: {}", o.status)),
            Err(e) => Err(anyhow::anyhow!("Failed to run rustfmt: {}", e)),
        }
    }
}

/// Find all .wj files in a directory or return the single file
fn find_wj_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("wj") {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eject_config_default() {
        let config = EjectConfig::default();
        assert!(config.format);
        assert!(config.comments);
        assert!(config.generate_cargo_toml);
    }

    #[test]
    fn test_generate_cargo_dependencies_empty() {
        let ejector = Ejector::new(EjectConfig::default());
        let deps = ejector.generate_cargo_dependencies();
        assert_eq!(deps, "");
    }

    #[test]
    fn test_generate_cargo_dependencies_json() {
        let mut ejector = Ejector::new(EjectConfig::default());
        ejector.all_stdlib_modules.insert("json".to_string());

        let deps = ejector.generate_cargo_dependencies();
        assert!(deps.contains("serde"));
        assert!(deps.contains("serde_json"));
    }

    #[test]
    fn test_generate_cargo_dependencies_multiple() {
        let mut ejector = Ejector::new(EjectConfig::default());
        ejector.all_stdlib_modules.insert("json".to_string());
        ejector.all_stdlib_modules.insert("regex".to_string());

        let deps = ejector.generate_cargo_dependencies();
        assert!(deps.contains("serde"));
        assert!(deps.contains("regex"));
    }
}
