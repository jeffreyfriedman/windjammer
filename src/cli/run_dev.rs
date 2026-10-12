//! `wj run`: interpret by default (instant start), fall back to the compiled path with a one-line notice
//! when the program uses a feature the interpreter does not run (ADR-0017). `--compiled`/`--release`
//! force the Rust path; `--interpret` forces the interpreter (no fallback).
//!
//! Project discovery (`wj.toml` upward, `--bin`, `cmd/<name>/main.wj`, `src/bin/<name>.wj`) is
//! `module_system::project`, shared with the interpreter's loader, so both paths run the same entry.

use crate::interpreter::runner::{self, RunOutcome};
use crate::module_system::project::{resolve_run_target, RunTarget};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Interpret; compiled fallback on a preflight-detected unsupported feature.
    Auto,
    /// Interpreter only (exit 2 on unsupported).
    Interpret,
    /// Transpile + cargo. `release` builds with optimizations.
    Compiled { release: bool },
}

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub path: Option<PathBuf>,
    pub bin: Option<String>,
    pub args: Vec<String>,
    pub mode: Mode,
    pub target: String,
    /// Called with the entry file before interpreting (the binary passes `wj build`'s lint report).
    pub lint: Option<fn(&Path)>,
}

/// Run once; returns the process exit status.
pub fn execute(opts: &RunOptions) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_default();
    let target = match resolve_run_target(opts.path.as_deref(), &cwd, opts.bin.as_deref()) {
        Ok(t) => t,
        Err(m) => {
            eprintln!("error: {}", m);
            return 1;
        }
    };
    let non_rust = !matches!(opts.target.to_lowercase().as_str(), "rust");
    if let (Some(lint), false, false) = (opts.lint, non_rust, matches!(opts.mode, Mode::Compiled { .. })) {
        lint(&target.entry);
    }
    match opts.mode {
        Mode::Compiled { release } => run_compiled(&target, &opts.args, &opts.target, release),
        _ if non_rust => run_compiled(&target, &opts.args, &opts.target, false),
        Mode::Interpret => match runner::run_entry(&target.entry, opts.args.clone(), false) {
            RunOutcome::Exited(c) => c,
            RunOutcome::NeedsCompiled(_) => 2,
        },
        Mode::Auto => match runner::run_entry(&target.entry, opts.args.clone(), true) {
            RunOutcome::Exited(c) => c,
            RunOutcome::NeedsCompiled(e) => {
                eprintln!("{}", fallback_notice(&e));
                run_compiled(&target, &opts.args, &opts.target, false)
            }
        },
    }
}

/// The one-line notice printed (to stderr) when `wj run` switches to the compiled path.
pub fn fallback_notice(e: &crate::interpreter::RuntimeError) -> String {
    let at = e.span.as_ref().map(|s| format!(" ({})", s)).unwrap_or_default();
    format!("wj: running compiled (cargo): {}{}; use `wj run --compiled` to skip this check", e.message, at)
}

/// What to hand the compiler for a run target: the source root when the entry is its crate root
/// (`main.wj`), the file itself for a loose script.
fn build_input(t: &RunTarget) -> Result<PathBuf, String> {
    let is_main = t.entry.file_name().is_some_and(|n| n == "main.wj");
    let parent = t.entry.parent().map(Path::to_path_buf).unwrap_or_default();
    let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    if is_main && canon(&parent) == canon(&t.source_root) {
        return Ok(t.source_root.clone());
    }
    if t.project_root.is_some() && canon(&parent) != canon(&t.source_root) {
        return Err(format!(
            "compiled runs of `{}` are not supported yet: a compiled binary needs its own directory with a main.wj (e.g. cmd/<name>/main.wj); `wj run --interpret` runs it",
            t.entry.display()
        ));
    }
    Ok(t.entry.clone())
}

fn run_compiled(t: &RunTarget, args: &[String], target: &str, release: bool) -> i32 {
    let input = match build_input(t) {
        Ok(p) => p,
        Err(m) => {
            eprintln!("error: {}", m);
            return 1;
        }
    };
    match super::run::execute_status(&input, args, target, release) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {:#}", e);
            1
        }
    }
}
