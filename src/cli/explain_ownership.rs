// wj explain-ownership - report inferred ownership, clones and widths with rule IDs.
//
// Reporting only (ADR-0001): the compiler is run as a child process to obtain the emitted
// Rust, so no compiler state is shared with the analysis shown here.

use crate::ir::explain::{explain_source, render, ExplainOptions};
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

fn emitted_rust(file: &Path) -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let dir = tempfile::tempdir().ok()?;
    let status = Command::new(exe)
        .arg("build")
        .arg(file)
        .args(["--no-cargo", "--no-lint", "--quiet", "-o"])
        .arg(dir.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    let stem = file.file_stem()?.to_str()?;
    std::fs::read_to_string(dir.path().join(format!("{stem}.rs"))).ok()
}

pub fn execute(file: &Path, function: Option<&str>, json: bool) -> Result<()> {
    let source = std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let opts = ExplainOptions {
        only_fn: function.map(String::from),
        emitted_rust: emitted_rust(file),
    };
    let report = explain_source(&source, &file.display().to_string(), &opts)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if json {
        println!("{}", render::to_json(&report));
    } else {
        print!("{}", render::to_text(&report));
    }
    Ok(())
}
