// wj check - front-end check of .wj sources (no cargo, no rustc).
//
// Thin driver over `crate::check`; diagnostics follow docs/diagnostics-schema.json.

use anyhow::Result;
use std::path::Path;
use std::time::Duration;

use crate::check::{self, CheckOptions, Checked};
use crate::diagnostics::render;

pub fn execute(path: Option<&Path>, json: bool, watch: bool, fix: bool) -> Result<()> {
    let root = path.unwrap_or_else(|| Path::new("."));
    if path.is_none() && Path::new("Cargo.toml").exists() && !has_wj_files(root) {
        return cargo_check();
    }
    if watch {
        return watch_loop(root, json, fix);
    }
    let ok = run_once(root, json, fix)?;
    if !ok {
        std::process::exit(1);
    }
    Ok(())
}

/// The lexer reports bad characters by panicking; the checker catches that and reports a
/// diagnostic, so the default panic message would only be noise.
fn check_quietly(root: &Path) -> Result<Checked> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = check::check_path(root, &CheckOptions::default());
    std::panic::set_hook(previous);
    result
}

fn run_once(root: &Path, json: bool, fix: bool) -> Result<bool> {
    let mut checked = check_quietly(root)?;
    if checked.report.summary.files_checked == 0 {
        checked.report = check_path(root);
    }
    let mut fixed = None;
    if fix {
        let outcome = check::apply_fixes(&checked.report.diagnostics, &checked.sources, true)?;
        if outcome.files_changed > 0 {
            checked = check_quietly(root)?;
        }
        fixed = Some(outcome);
    }
    if json {
        println!("{}", checked.report.to_json());
    } else {
        print_human(&checked);
        if let Some(f) = fixed {
            println!(
                "wj check --fix: {} edit(s) applied in {} file(s){}",
                f.edits_applied,
                f.files_changed,
                if f.files_skipped.is_empty() {
                    String::new()
                } else {
                    format!(", {} file(s) skipped (fix would not parse)", f.files_skipped.len())
                }
            );
        }
    }
    Ok(checked.report.success)
}

fn print_human(c: &Checked) {
    let source_of = |f: &str| c.sources.get(f).cloned();
    for d in &c.report.diagnostics {
        println!("{}", render::render(d, &source_of));
    }
    let s = &c.report.summary;
    println!(
        "wj check: {} error(s), {} warning(s) in {} file(s) ({} ms{})",
        s.errors,
        s.warnings,
        s.files_checked,
        s.elapsed_ms,
        if s.files_cached > 0 { ", cached" } else { "" }
    );
}

fn watch_loop(root: &Path, json: bool, fix: bool) -> Result<()> {
    loop {
        let before = check::watch::snapshot(root);
        let _ = run_once(root, json, fix)?;
        if !json {
            eprintln!("watching {} for changes (Ctrl-C to stop)...", root.display());
        }
        check::watch::wait_for_change(root, &before, Duration::from_millis(250));
    }
}

/// Gate for `wj run --interpret`: the interpreter must not be more permissive than the compiler.
/// Runs the same front-end checks as `wj check`; on any error-level diagnostic prints them and
/// returns `Err` so the caller refuses to execute the program. Warnings do not block.
pub fn require_clean(path: &Path, lints_block: bool) -> Result<()> {
    // Cached only inside a project (`wj.toml`): results go to its `.wj-cache/`, the build's cache directory; a
    // lone script never gets a `.wj-cache` written next to it. The cache never changes diagnostics (ADR-0020,
    // round 3 rung 4; gate: tests/check_cache_differential_test.rs).
    let use_cache = check::effects::manifest_path(path).is_some();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let checked = check::check_path(path, &CheckOptions { use_cache });
    std::panic::set_hook(previous);
    let checked = checked?;
    // `--no-lint`: Rust-leakage lints (W####) do not stop the run; every other error does.
    // a lint code is `W` and four digits; `WJ0003` is a compile error and must stop the run whatever `--no-lint` says
    let is_lint = |code: &str| code.len() == 5 && code.starts_with('W') && code[1..].bytes().all(|b| b.is_ascii_digit());
    let blocks = |d: &crate::diagnostics::Diagnostic| {
        d.severity == crate::diagnostics::Severity::Error && (lints_block || !is_lint(&d.code))
    };
    let errors = checked.report.diagnostics.iter().filter(|d| blocks(d)).count();
    if errors == 0 {
        return Ok(());
    }
    let source_of = |f: &str| checked.sources.get(f).cloned();
    for d in checked.report.diagnostics.iter().filter(|d| blocks(d)) {
        eprintln!("{}", render::render(d, &source_of));
    }
    anyhow::bail!(
        "not running {}: `wj check` reports {} error(s); the interpreter runs only programs the compiler accepts",
        path.display(),
        errors
    )
}

/// Programmatic entry used by tools (MCP `check_code`): the shared-schema report for `path`.
/// A path with no `.wj` files yields a failed report with one error.
pub fn check_path(path: &Path) -> crate::diagnostics::Report {
    match check::check_path(path, &CheckOptions { use_cache: false }) {
        Ok(c) if c.report.summary.files_checked > 0 => c.report,
        Ok(_) => crate::diagnostics::Report::new(
            "check",
            vec![crate::diagnostics::Diagnostic::error(
                "WJ0011",
                format!("no .wj files found at {}", path.display()),
            )],
        ),
        Err(e) => crate::diagnostics::Report::new(
            "check",
            vec![crate::diagnostics::Diagnostic::error("WJ0011", format!("{e}"))],
        ),
    }
}

fn has_wj_files(root: &Path) -> bool {
    check::watch::snapshot(root).keys().next().is_some()
}

/// A plain Rust crate (a `Cargo.toml` and no `.wj` sources) keeps the old behaviour.
fn cargo_check() -> Result<()> {
    let status = std::process::Command::new("cargo").arg("check").status()?;
    if !status.success() {
        anyhow::bail!("Type check failed");
    }
    Ok(())
}
