//! `wj run`: interpret by default (instant start), fall back to the compiled path with a one-line notice
//! when the program uses a feature the interpreter does not run (ADR-0019). `--compiled`/`--release`
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

#[derive(Clone)]
pub struct RunOptions {
    pub path: Option<PathBuf>,
    pub bin: Option<String>,
    pub args: Vec<String>,
    pub mode: Mode,
    pub target: String,
    /// Apply edits to the running program (ADR-0021); interpreter only.
    pub hot: bool,
    /// Interpreter knobs (scheduler seed, deterministic time, network policy).
    pub interp: runner::RunOptions,
    /// Called with the entry file before interpreting (the binary passes `wj build`'s lint report).
    pub lint: Option<fn(&Path)>,
    /// Interpreted runs: lints (W####) are reported but do not stop the run; compile errors still do.
    pub no_lint: bool,
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
        Mode::Interpret | Mode::Auto => {
            // The interpreter must not be more permissive than the compiler: same front end as `wj check`.
            if let Err(e) = crate::cli::check::require_clean(&target.entry, !opts.no_lint) {
                eprintln!("Error: {e}");
                return 1;
            }
            let auto = opts.mode == Mode::Auto;
            let host = match host_bridge(&target) {
                Ok(h) => h,
                Err(m) if auto => {
                    eprintln!("wj: running compiled (cargo): {}; use `wj run --compiled` to skip this check", m);
                    return run_compiled(&target, &opts.args, &opts.target, false);
                }
                Err(m) => {
                    eprintln!("error: unsupported in the interpreter: {}", m);
                    return 2;
                }
            };
            let mut interp = opts.interp.clone();
            interp.durable_sites = crate::check::durable_sites::discover(&target.entry).unwrap_or_default();
            let eo = runner::EntryOptions { fallback: auto, host, hot: opts.hot, run: interp };
            match runner::run_entry(&target.entry, opts.args.clone(), eo) {
                RunOutcome::Exited(c) => c,
                RunOutcome::NeedsCompiled(e) => {
                    eprintln!("{}", fallback_notice(&e));
                    run_compiled(&target, &opts.args, &opts.target, false)
                }
            }
        }
    }
}

/// Build (or reuse) the host bridge when the program declares `extern fn`s and the project has Rust
/// sources for them (ADR-0020). `Err` names the item that cannot be bridged.
fn host_bridge(t: &RunTarget) -> Result<Option<(PathBuf, Vec<crate::interpreter::host_bridge::ExternSig>)>, String> {
    let Some(root) = t.project_root.clone() else { return Ok(None) };
    let rust = super::host_build::rust_sources(&root, &t.source_root);
    if rust.is_empty() {
        return Ok(None);
    }
    let files = crate::interpreter::loader::program_files(&t.entry).map_err(|e| e.message)?;
    if !files.iter().any(|f| std::fs::read_to_string(&f.file).is_ok_and(|s| s.contains("extern"))) {
        return Ok(None);
    }
    let programs: Vec<&crate::parser::Program<'_>> = files.iter().map(|f| f.program).collect();
    let sigs = crate::interpreter::host_bridge::extern_signatures(&programs)?;
    if sigs.is_empty() {
        return Ok(None);
    }
    let lib = super::host_build::ensure(&root, &rust, &sigs, &mut |m| eprintln!("{}", m))?;
    Ok(Some((lib, sigs)))
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
