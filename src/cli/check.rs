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
