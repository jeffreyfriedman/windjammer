//! CLI execution utilities
//!
//! This module handles:
//! - Running compiled Windjammer programs
//! - Interpreting Windjammer files directly (tree-walking interpreter)
//! - REPL (Read-Eval-Print Loop) for interactive development

use anyhow::Result;
use std::path::Path;

use crate::CompilationTarget;
use crate::{build_project, interpreter};

pub fn run_file(file: &Path, target: CompilationTarget, args: &[String]) -> Result<()> {
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

/// Run a file or project directory in the interpreter: no cargo, no rustc, program stdout only.
pub fn interpret_file(file: &Path) -> Result<()> {
    interpret_with_args(file, Vec::new())
}

pub fn interpret_with_args(file: &Path, args: Vec<String>) -> Result<()> {
    report_build_lints(file);
    let code = interpreter::runner::run_path(file, args);
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

/// Run the Windjammerscript REPL (Read-Eval-Print Loop)
pub fn run_repl() -> Result<()> {
    interpreter::repl::run_repl_stdio();
    Ok(())
}

/// Print the lints `wj build` would apply (same `linter::lint_text` source of truth) before interpreting.
/// Interpretation still runs.
pub fn report_build_lints(file: &Path) {
    if file.is_file() {
        if let Ok(src) = std::fs::read_to_string(file) {
            let name = file.to_string_lossy().to_string();
            for d in crate::linter::lint_text(&name, &src).unwrap_or_default() {
                let would_fail = d.level == crate::linter::LintLevel::Error;
                eprintln!("{}{}", d, if would_fail { "  (error in `wj build`)" } else { "" });
            }
        }
    }
}
