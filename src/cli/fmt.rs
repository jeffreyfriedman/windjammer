// wj fmt - Format Windjammer (.wj) source files.
//
// Adapter between the filesystem/CLI and the pure `formatter` core. Options come from the
// nearest `wj.toml` `[fmt]` table. `--rust` keeps the legacy `cargo fmt --all` behavior for
// generated Rust crates.

use crate::formatter::{format_source, FormatOptions};
use anyhow::{bail, Context, Result};
use colored::*;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Entry point for `wj fmt`. A single path of `-` formats stdin to stdout.
pub fn execute(check: bool, paths: &[PathBuf], rust: bool) -> Result<()> {
    if rust {
        return execute_rust(check);
    }
    if paths.len() == 1 && paths[0] == Path::new("-") {
        return format_stdin(check);
    }
    let roots: Vec<PathBuf> = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.to_vec()
    };
    format_paths(&roots, check)
}

fn format_stdin(check: bool) -> Result<()> {
    let mut src = String::new();
    std::io::stdin().read_to_string(&mut src)?;
    let out = format_source(&src, &options_for_dir(Path::new(".")))?;
    if check {
        if out != src {
            bail!("stdin is not formatted");
        }
        return Ok(());
    }
    std::io::stdout().write_all(out.as_bytes())?;
    Ok(())
}

fn format_paths(roots: &[PathBuf], check: bool) -> Result<()> {
    let mut files = Vec::new();
    for root in roots {
        if !root.exists() {
            bail!("path not found: {}", root.display());
        }
        collect_wj_files(root, &mut files);
    }
    files.sort();
    let (mut changed, mut failed) = (Vec::new(), 0usize);
    for file in &files {
        match format_file(file, check) {
            Ok(true) => changed.push(file.clone()),
            Ok(false) => {}
            Err(e) => {
                eprintln!("{} {}: {:#}", "error".red().bold(), file.display(), e);
                failed += 1;
            }
        }
    }
    report(check, files.len(), &changed, failed)
}

fn report(check: bool, total: usize, changed: &[PathBuf], failed: usize) -> Result<()> {
    for f in changed {
        let verb = if check { "Would reformat" } else { "Formatted" };
        println!("{} {}", verb.yellow().bold(), f.display());
    }
    if failed > 0 {
        bail!("{failed} file(s) could not be formatted");
    }
    if check && !changed.is_empty() {
        bail!("{} file(s) need formatting", changed.len());
    }
    println!("{} {} file(s) checked", "✓".green().bold(), total);
    Ok(())
}

/// Format one file; returns whether its content differs from the formatted text.
fn format_file(path: &Path, check: bool) -> Result<bool> {
    let src = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let out = format_source(&src, &options_for_dir(dir))?;
    if out == src {
        return Ok(false);
    }
    if !check {
        fs::write(path, out).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(true)
}

/// Recursively gather `.wj` files, skipping build output and hidden directories.
fn collect_wj_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        out.push(path.to_path_buf());
        return;
    }
    let Ok(rd) = fs::read_dir(path) else { return };
    for entry in rd.flatten() {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if !name.starts_with('.') && name != "target" && name != "build" && name != "node_modules" {
                collect_wj_files(&p, out);
            }
        } else if p.extension().is_some_and(|e| e == "wj") {
            out.push(p);
        }
    }
}

/// Options from the nearest `wj.toml` at or above `dir`; defaults when none exists.
fn options_for_dir(dir: &Path) -> FormatOptions {
    let start = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    for ancestor in start.ancestors() {
        if let Ok(text) = fs::read_to_string(ancestor.join("wj.toml")) {
            return FormatOptions::from_toml_str(&text);
        }
    }
    FormatOptions::default()
}

/// Legacy behavior: `cargo fmt --all` for generated Rust crates.
fn execute_rust(check: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("fmt").arg("--all");
    if check {
        cmd.arg("--").arg("--check");
    }
    if !cmd.status()?.success() {
        bail!("Formatting check failed");
    }
    Ok(())
}
