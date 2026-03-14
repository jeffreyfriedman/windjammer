//! Minimal wj-lint binary - Rust leakage linter for .wj files
//!
//! Uses only windjammer lib (no cli module). For CI and pre-commit hooks.
//!
//! Usage: wj-lint [--strict] <path>

use std::env;
use std::fs;
use std::path::Path;

use windjammer::lexer::Lexer;
use windjammer::linter::rust_leakage::RustLeakageLinter;
use windjammer::parser::Parser;

fn lint_file(path: &Path, strict: bool) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let file_name = path.to_string_lossy().to_string();

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, file_name.clone(), source);
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    let mut linter = RustLeakageLinter::new(&file_name);
    linter.lint_program(&program);
    let warnings = linter.into_diagnostics();

    if warnings.is_empty() {
        println!("✓ {}: No issues found", path.display());
        return Ok(());
    }

    for warning in &warnings {
        println!("{}", warning);
    }
    println!("\n⚠  {} warning(s) found in {}", warnings.len(), path.display());

    if strict {
        return Err(format!("Linter failed: {} warning(s) found", warnings.len()).into());
    }
    Ok(())
}

fn collect_wj_files(path: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()).into());
    }
    if path.is_file() {
        if path.extension().map_or(false, |e| e == "wj") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let p = entry?.path();
            if p.is_dir() {
                collect_wj_files(&p, files)?;
            } else if p.extension().map_or(false, |e| e == "wj") {
                files.push(p);
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut strict = false;
    let mut path = None;

    for arg in args.iter().skip(1) {
        if arg == "--strict" {
            strict = true;
        } else if !arg.starts_with('-') {
            path = Some(Path::new(arg).to_path_buf());
            break;
        }
    }

    let path = path.ok_or("Usage: wj-lint [--strict] <path>")?;

    let mut files = Vec::new();
    collect_wj_files(&path, &mut files)?;

    if files.is_empty() {
        return Err(format!("No .wj files found at {}", path.display()).into());
    }

    let mut failed = false;
    for file in &files {
        if lint_file(file, strict).is_err() {
            failed = true;
        }
    }

    if failed {
        std::process::exit(1);
    }
    Ok(())
}
