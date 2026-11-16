// wj check - front-end check of .wj sources (no cargo, no rustc).
//
// Thin driver over `crate::check`; diagnostics follow docs/diagnostics-schema.json.

use anyhow::Result;
use std::path::Path;
use std::time::Duration;

use crate::check::{self, CheckOptions, Checked};
use crate::diagnostics::render;

pub fn execute(path: Option<&Path>, json: bool, watch: bool, fix: bool, format: render::Format) -> Result<()> {
    let root = path.unwrap_or_else(|| Path::new("."));
    if let Some(manifest) = check::effects::manifest_path(root) {
        if let Some(dir) = manifest.parent() {
            for m in crate::package::graph::missing_configurable_report(dir) {
                eprintln!("wj check: {m}");
            }
        }
    }
    if path.is_none() && Path::new("Cargo.toml").exists() && !has_wj_files(root) {
        return cargo_check();
    }
    if watch {
        return watch_loop(root, json, fix, format);
    }
    let ok = run_once(root, json, fix, format)?;
    if !ok {
        std::process::exit(1);
    }
    Ok(())
}

/// The per-file analysis cache shared by every checkout of this user (`<cache root>/analysis`, content-keyed:
/// `check/shared_cache.rs`).
fn share_analysis() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let store = AnalysisDir(crate::cargo_cache::entries::AnalysisStore { dir: crate::cargo_cache::entries::analysis_dir() });
        check::shared_cache::install(std::sync::Arc::new(store));
    });
}

/// The scanned runtime signature layer, shared by every process of this compiler build (`<cache root>/registry`,
/// `analyzer/signature_registry/snapshot.rs`). Needs no project, so every check installs it.
fn share_registry_layer() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let layer = crate::cargo_cache::entries::RegistryStore { dir: crate::cargo_cache::entries::registry_dir() };
        if let Some(compiler) = check::shared_cache::compiler_key() {
            crate::analyzer::install_layer_store(std::sync::Arc::new(RegistryDir(layer)), compiler);
        }
    });
}

struct RegistryDir(crate::cargo_cache::entries::RegistryStore);

impl crate::analyzer::LayerStore for RegistryDir {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key)
    }
    fn put(&self, key: &str, text: &str) {
        self.0.put(key, text)
    }
    fn get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        self.0.get_bytes(key)
    }
    fn put_bytes(&self, key: &str, bytes: &[u8]) {
        self.0.put_bytes(key, bytes)
    }
}

struct AnalysisDir(crate::cargo_cache::entries::AnalysisStore);

impl check::shared_cache::SharedStore for AnalysisDir {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key)
    }
    fn put(&self, key: &str, text: &str) {
        self.0.put(key, text)
    }
}

/// The lexer reports bad characters by panicking; the checker catches that and reports a
/// diagnostic, so the default panic message would only be noise.
fn check_quietly(root: &Path) -> Result<Checked> {
    share_registry_layer();
    share_analysis();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = check::check_path(root, &CheckOptions::default());
    std::panic::set_hook(previous);
    result.map(|c| super::widths_gate::add_to(c, root))
}

fn run_once(root: &Path, json: bool, fix: bool, format: render::Format) -> Result<bool> {
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
        print_human(&checked, format);
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

fn print_human(c: &Checked, format: render::Format) {
    let source_of = |f: &str| c.sources.get(f).cloned();
    for d in &c.report.diagnostics {
        println!("{}", render::render_as(d, &source_of, format));
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

fn watch_loop(root: &Path, json: bool, fix: bool, format: render::Format) -> Result<()> {
    loop {
        let before = check::watch::snapshot(root);
        let _ = run_once(root, json, fix, format)?;
        if !json {
            eprintln!("watching {} for changes (Ctrl-C to stop)...", root.display());
        }
        check::watch::wait_for_change(root, &before, Duration::from_millis(250));
    }
}

/// Gate for `wj run --interpret`: the interpreter must not be more permissive than the compiler.
/// Apply the machine-applicable fixes of the front-end check to the sources on disk (`wj run --fix`); returns the
/// number of edits applied.
pub fn fix_in_place(path: &Path) -> Result<usize> {
    let checked = check_quietly(path)?;
    Ok(check::apply_fixes(&checked.report.diagnostics, &checked.sources, true)?.edits_applied)
}

/// Runs the same front-end checks as `wj check`; on any error-level diagnostic prints them and
/// returns `Err` so the caller refuses to execute the program. Warnings do not block.
pub fn require_clean(path: &Path, lints_block: bool, format: render::Format) -> Result<()> {
    // Cached only inside a project (`wj.toml`): results go to its `.wj-cache/`, the build's cache directory; a
    // lone script never gets a `.wj-cache` written next to it. The cache never changes diagnostics (ADR-0020,
    // round 3 rung 4; gate: tests/check_cache_differential_test.rs).
    let use_cache = check::effects::manifest_path(path).is_some();
    share_registry_layer();
    if use_cache {
        share_analysis();
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    crate::debug_env::phase("check-start");
    let checked = check::check_path(path, &CheckOptions { use_cache });
    std::panic::set_hook(previous);
    crate::debug_env::phase("check-path");
    let checked = checked?;
    // `--no-lint`: Rust-leakage lints (W####) do not stop the run; every other error does.
    // a lint code is `W` and four digits; `WJ0003` is a compile error and must stop the run whatever `--no-lint` says
    let is_lint = |code: &str| code.len() == 5 && code.starts_with('W') && code[1..].bytes().all(|b| b.is_ascii_digit());
    let blocks = |d: &crate::diagnostics::Diagnostic| {
        d.severity == crate::diagnostics::Severity::Error && (lints_block || !is_lint(&d.code))
    };
    // uses after a move (WJ0007) are among them: `wj check` predicts them from the generated Rust (ADR-0030)
    let blocking: Vec<&crate::diagnostics::Diagnostic> = checked.report.diagnostics.iter().filter(|d| blocks(d)).collect();
    let errors = blocking.len();
    // paths relative to the current directory: shorter for people and agents (sources looked up either way)
    let cwd = std::env::current_dir().ok().map(|d| d.to_string_lossy().to_string() + "/");
    let short = |f: &str| cwd.as_deref().and_then(|c| f.strip_prefix(c)).unwrap_or(f).to_string();
    let source_of = |f: &str| {
        let full = cwd.as_deref().map(|c| format!("{c}{f}"));
        checked.sources.get(f).or_else(|| full.as_deref().and_then(|g| checked.sources.get(g))).cloned().or_else(|| std::fs::read_to_string(f).ok())
    };
    if errors == 0 {
        // ADR-0033: integer widths the compiled build would mix are a backend defect, not a language error. `wj check`
        // reports them as errors; an interpreted run warns before the program's output and runs it (scripts that
        // are only ever interpreted keep working)
        for mut d in super::widths_gate::predicted_widths(path) {
            d.severity = crate::diagnostics::Severity::Warning;
            d.message = d.message.replacen("the compiled build would reject this", "this runs interpreted, but the compiled build would reject it", 1);
            for sp in d.primary_span.iter_mut() {
                sp.file = short(&sp.file);
            }
            eprintln!("{}", render::render_as(&d, &source_of, format));
        }
        return Ok(());
    }
    for d in blocking {
        let mut d = d.clone();
        for sp in d.primary_span.iter_mut().chain(d.secondary_spans.iter_mut()) {
            sp.file = short(&sp.file);
        }
        eprintln!("{}", render::render_as(&d, &source_of, format));
    }
    anyhow::bail!(
        "not running {}: `wj check` reports {} error(s); the interpreter runs only programs the compiler accepts",
        short(&path.display().to_string()),
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
