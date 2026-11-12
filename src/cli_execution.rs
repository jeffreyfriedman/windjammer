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
    report_build_lints(file, false);
    let code = interpreter::runner::run_path(file, args);
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

/// Run the Windjammerscript REPL (Read-Eval-Print Loop)
pub fn run_repl() -> Result<()> {
    // The legacy driver has no Rust-crate host (the `wj` driver wires it, ADR-0031): importing `cli` here broke the layering.
    crate::interpreter::repl::run_repl_with(None, None);
    Ok(())
}

/// Print the lints `wj build` would apply (same `linter::lint_text` source of truth) before interpreting.
/// Interpretation still runs. `gate_reports_errors`: the check gate prints error-level lints itself (with the
/// source line) and stops the run, so they are not printed twice. When stderr is not a terminal (an agent or a
/// tool reading the output), warnings are grouped per lint code on one line each.
pub fn report_build_lints(file: &Path, gate_reports_errors: bool) {
    use std::io::IsTerminal;
    if !file.is_file() {
        return;
    }
    let Ok(src) = std::fs::read_to_string(file) else { return };
    let name = file.to_string_lossy().to_string();
    let lints: Vec<_> = crate::linter::lint_text(&name, &src)
        .unwrap_or_default()
        .into_iter()
        .filter(|d| !(gate_reports_errors && d.level == crate::linter::LintLevel::Error))
        .collect();
    let compact = !std::io::stderr().is_terminal();
    if !compact {
        for d in &lints {
            let would_fail = d.level == crate::linter::LintLevel::Error;
            eprintln!("{}{}", d, if would_fail { "  (error in `wj build`)" } else { "" });
        }
        return;
    }
    for line in compact_lints(&lints) {
        eprintln!("{line}");
    }
}

/// One line per lint code: level, code, message, the first locations, and the first help.
pub fn compact_lints(lints: &[crate::linter::LintDiagnostic]) -> Vec<String> {
    let mut order: Vec<&str> = Vec::new();
    for d in lints {
        if d.level != crate::linter::LintLevel::Allow && !order.contains(&d.lint_name.as_str()) {
            order.push(&d.lint_name);
        }
    }
    order
        .into_iter()
        .map(|code| {
            let all: Vec<_> = lints.iter().filter(|d| d.lint_name == code).collect();
            let first = all[0];
            let level = if first.level == crate::linter::LintLevel::Error { "error" } else { "warning" };
            let file = std::path::Path::new(&first.location.file).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
            let mut at: Vec<String> = all.iter().take(4).map(|d| format!("{}:{}", d.location.line, d.location.column)).collect();
            if all.len() > 4 {
                at.push(format!("+{} more", all.len() - 4));
            }
            let help = first.help.as_deref().map(|h| format!(" (help: {h})")).unwrap_or_default();
            format!("{level}[{code}] {}: {file} {}{help}", first.message, at.join(", "))
        })
        .collect()
}
