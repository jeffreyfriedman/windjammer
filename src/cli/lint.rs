// wj lint - Run Windjammer Rust leakage linter on .wj files
//
// Detects W0001-W0004: explicit &/&mut, .unwrap(), .iter(), explicit borrows.
// Use --strict to fail on warnings (for CI).

use anyhow::{bail, Result};
use colored::*;
use std::fs;
use std::path::Path;

/// CLI entry point - lint path (file or directory)
pub fn execute(path: &Path, strict: bool) -> Result<()> {
    lint_path(path, strict)
}

use crate::lexer::Lexer;
use crate::linter::rust_leakage::RustLeakageLinter;
use crate::parser::Parser;

/// Lint a single .wj file for Rust leakage
pub fn lint_file(path: &Path, strict: bool) -> Result<()> {
    let source = fs::read_to_string(path)?;
    let file_name = path.to_string_lossy().to_string();

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, file_name.clone(), source);
    let program = parser.parse().map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    let mut linter = RustLeakageLinter::new(&file_name);
    linter.lint_program(&program);
    let warnings = linter.into_diagnostics();

    if warnings.is_empty() {
        println!("{} {}: No issues found", "✓".green().bold(), path.display());
        return Ok(());
    }

    // Print warnings
    for warning in &warnings {
        println!("{}", warning);
    }

    println!(
        "\n{} {} warning(s) found in {}",
        "⚠".yellow().bold(),
        warnings.len(),
        path.display()
    );

    if strict {
        bail!("Linter failed: {} warning(s) found", warnings.len());
    }

    Ok(())
}

fn collect_wj_files(path: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }
    if path.is_file() {
        if path.extension().map_or(false, |e| e == "wj") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {
                collect_wj_files(&p, files)?;
            } else if p.extension().map_or(false, |e| e == "wj") {
                files.push(p);
            }
        }
    }
    Ok(())
}

/// Lint a path (file or directory) - recursively finds .wj files
pub fn lint_path(path: &Path, strict: bool) -> Result<()> {
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }

    let mut files = Vec::new();
    collect_wj_files(path, &mut files)?;

    if files.is_empty() {
        bail!("No .wj files found at {}", path.display());
    }

    let mut failed = false;
    for file in &files {
        if lint_file(file, strict).is_err() {
            failed = true;
        }
    }

    if failed {
        bail!("Linter found issues in one or more files");
    }

    Ok(())
}
