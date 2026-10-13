//! `wj build --json`: the shared diagnostics schema for builds.
//!
//! Order: front-end check (structured, no rustc) -> codegen -> optionally `cargo build` with
//! rustc errors translated through the error mapper. Exit status is non-zero on any error.

use crate::check::{self, CheckOptions};
use crate::diagnostics::rustc_map::{self, SourceCache};
use crate::diagnostics::{Diagnostic, Report, Severity};
use anyhow::Result;
use std::path::Path;

pub struct JsonBuild<'a> {
    pub path: &'a Path,
    pub output_dir: &'a Path,
    pub target_str: &'a str,
    pub run_cargo_check: bool,
    pub library: bool,
    pub enable_lint: bool,
    pub config: &'a crate::compiler_config::CompilerConfig,
    pub external_metadata: &'a [(&'a str, &'a Path)],
}

fn emit(mut report: Report, files: usize) -> ! {
    report.summary.files_checked = files;
    println!("{}", report.to_json());
    std::process::exit(if report.success { 0 } else { 1 });
}

pub fn run(b: JsonBuild<'_>) -> Result<()> {
    let checked = check::check_path(b.path, &CheckOptions::default())?;
    let files = checked.report.summary.files_checked;
    let mut diags = checked.report.diagnostics.clone();
    if b.target_str.to_lowercase() != "rust" {
        diags.push(Diagnostic::error(
            "WJ0028",
            format!("JSON output mode only supports the Rust target, got: {}", b.target_str),
        ));
        emit(Report::new("build", diags), files);
    }
    if diags.iter().any(|d| d.is_error()) {
        emit(Report::new("build", diags), files);
    }
    let built = crate::build_project_with_config(
        b.path,
        b.output_dir,
        crate::CompilationTarget::Rust,
        b.enable_lint,
        b.library,
        b.external_metadata,
        b.config,
    );
    if let Err(e) = built {
        diags.push(Diagnostic::error("WJ0028", format!("{e}")));
    } else if b.run_cargo_check {
        diags.extend(cargo_diagnostics(b.output_dir));
    }
    emit(Report::new("build", diags), files)
}

/// Run `cargo build` in the generated crate and translate every rustc error.
fn cargo_diagnostics(output_dir: &Path) -> Vec<Diagnostic> {
    let out = std::process::Command::new("cargo")
        .args(["build", "--message-format=json"])
        .current_dir(output_dir)
        .output();
    let Ok(out) = out else {
        return vec![Diagnostic::error("WJ0099", "could not run cargo to check the generated code")];
    };
    if out.status.success() {
        return Vec::new();
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let maps = super::build::load_source_maps(output_dir).unwrap_or_else(|_| crate::source_map::SourceMap::new());
    let mapped = crate::error_mapper::ErrorMapper::new(maps).map_rustc_output(&text);
    let mut cache = SourceCache::default();
    let diags: Vec<Diagnostic> = mapped
        .iter()
        .filter(|d| d.level == crate::error_mapper::DiagnosticLevel::Error)
        .map(|d| rustc_map::from_windjammer(d, &mut cache))
        .collect();
    if diags.is_empty() {
        return vec![Diagnostic::new(
            "WJ0099",
            Severity::Error,
            "the generated code failed to compile and the error could not be parsed",
        )];
    }
    diags
}
