//! `wj check`: the front end of the compiler as a standalone, rustc-free checker.
//! See `MODULE.md` and `docs/DIAGNOSTICS.md`.

mod ambient;
mod cache;
mod dep_sources;
pub mod durable_sites;
pub mod effects;
pub mod files;
mod file_cache;
mod fix;
pub(crate) mod front;
mod test_api;
pub use test_api::{compiler_identity_of, uninstall_shared_cache};
mod borrow_moves;
pub mod moves;
pub mod locate;
pub mod local_mut;
mod project;
mod rust_calls;
mod rust_surface_health;
mod match_variants;
pub mod run_cache;
pub mod shared_cache;
mod std_calls;
pub mod taint;
mod tasks;
mod semantic;
pub mod watch;
pub mod widths;

use crate::diagnostics::{Diagnostic, Report};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

pub use fix::{apply_fixes, FixOutcome};

#[derive(Debug, Clone, Copy)]
pub struct CheckOptions {
    pub use_cache: bool,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self { use_cache: true }
    }
}

pub struct Checked {
    pub report: Report,
    /// Source text of every checked file, for rendering excerpts.
    pub sources: HashMap<String, String>,
}

/// Check a `.wj` file or a project directory. No cargo, no rustc, no generated code.
pub fn check_path(path: &Path, opts: &CheckOptions) -> Result<Checked> {
    let start = Instant::now();
    crate::debug_env::phase("check-inputs-start");
    let (sources, keyed) = inputs(path)?;
    crate::debug_env::phase("check-inputs");
    let fp = cache::fingerprint(&cache::relative_inputs(path, &keyed));
    let cached = if opts.use_cache { cache::load(path, &fp) } else { None };
    let was_cached = cached.is_some();
    let diagnostics = match cached {
        Some(d) => d,
        None => {
            let r = run_with(path, &sources, opts.use_cache);
            if opts.use_cache {
                cache::store(path, &fp, r.use_depth, r.ambient.as_deref(), &r.diagnostics);
            }
            r.diagnostics
        }
    };
    let mut report = Report::new("check", diagnostics);
    report.summary.files_checked = sources.len();
    report.summary.files_cached = if was_cached { sources.len() } else { 0 };
    report.summary.elapsed_ms = start.elapsed().as_millis() as u64;
    Ok(Checked {
        report,
        sources: sources.into_iter().collect(),
    })
}

/// The program at `path`: its `.wj` sources, and every input of a check of it (sources, the manifest, the lock
/// file, dependency sources, Rust crate surfaces), as (path, text) pairs.
fn inputs(path: &Path) -> Result<(Vec<(String, String)>, Vec<(String, String)>)> {
    let paths = files::collect(path)?;
    let mut sources: Vec<(String, String)> = Vec::with_capacity(paths.len());
    for p in &paths {
        sources.push((p.display().to_string(), std::fs::read_to_string(p)?));
    }
    // inputs that change what the effect pass reports: the manifest, the lock file and dependency sources, Rust crate surfaces (ADR-0026)
    let mut keyed = sources.clone();
    if let Some(manifest) = effects::manifest_path(path) {
        let dir = manifest.parent().unwrap_or(path).to_path_buf();
        for extra in [manifest.clone(), dir.join(crate::ir::effect_facts::lock::LOCK_FILE)].into_iter().chain(dep_sources::input_paths(&dir)).chain(crate::package::rust_surface::surface_files(&dir)) {
            if let Ok(t) = std::fs::read_to_string(&extra) {
                keyed.push((extra.display().to_string(), t));
            }
        }
    }
    Ok((sources, keyed))
}

/// Every input of the program at `path` (see `inputs`) with canonical paths, so that two spellings of one
/// program (`main.wj`, `./main.wj`, an absolute path) give the same list. Used to key compiled verdicts.
pub fn program_inputs(path: &Path) -> Result<Vec<(String, String)>> {
    let (_, keyed) = inputs(path)?;
    let canon = |f: String| std::fs::canonicalize(&f).map(|p| p.display().to_string()).unwrap_or(f);
    let mut out: Vec<(String, String)> = keyed.into_iter().map(|(f, t)| (canon(f), t)).collect();
    out.sort();
    Ok(out)
}

/// Identity of this compiler build (version, executable path, size and modification time), the same one the
/// check caches use.
pub fn compiler_identity() -> String {
    cache::identity()
}

/// ADR-0026: errors in calls into Rust crates that have a signature surface (and in the surfaces themselves),
/// what `wj build` stops on before generating code. Cheap when the project has no surface (no file is read).
pub fn rust_crate_calls(path: &Path) -> Result<Checked> {
    let crates = effects::manifest_path(path)
        .and_then(|m| m.parent().map(crate::package::rust_surface::load))
        .unwrap_or_default();
    let mut out = Vec::new();
    let mut sources: Vec<(String, String)> = Vec::new();
    if !crates.surfaces.is_empty() {
        for p in files::collect(path)? {
            sources.push((p.display().to_string(), std::fs::read_to_string(&p)?));
        }
        let mut sink = Vec::new();
        let parsed: Vec<front::Parsed> = sources.iter().filter_map(|(f, t)| front::parse(f, t, &mut sink)).collect();
        let refs: Vec<&front::Parsed> = parsed.iter().collect();
        let idx = semantic::Index::build(&refs.iter().map(|p| &p.program).collect::<Vec<_>>());
        rust_calls::run(&crates, &idx, &refs, &mut out);
    }
    for s in &crates.surfaces {
        sources.push((s.path.display().to_string(), s.text.clone()));
    }
    Ok(Checked { report: Report::new("check", finish(out)), sources: sources.into_iter().collect() })
}

/// ADR-0002: the `WJ0051` diagnostics (local bindings changed without `mut`) of the program at `path`, exactly as
/// `wj check` reports them; `wj build` and `wj run` print these. No cache, no other check.
pub fn local_mutability(path: &Path) -> Result<Checked> {
    let (sources, _) = inputs(path)?;
    let found = local_mutability_sources(&sources);
    Ok(Checked { report: Report::new("check", found), sources: sources.into_iter().collect() })
}

/// The `WJ0006` diagnostics for bare `use` of a module that does not exist (`use state_model` with no such module
/// beside the file or in the project, in the standard library or in `wj.toml`), exactly as `wj check` reports them;
/// `wj build` prints these and stops. No cache, no other check.
pub fn missing_modules(path: &Path) -> Result<Checked> {
    let (sources, _) = inputs(path)?;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut sink = Vec::new();
    let parsed: Vec<front::Parsed> = sources.iter().filter_map(|(f, t)| front::parse(f, t, &mut sink)).collect();
    let found = project::module_imports_only(path, &parsed);
    std::panic::set_hook(previous);
    Ok(Checked { report: Report::new("check", finish(found)), sources: sources.into_iter().collect() })
}

/// The `WJ0035` diagnostics (an iterator used as a list) of the program at `path`, exactly as `wj check` reports them;
/// `wj build` prints these and stops. No cache, no other check.
pub fn iterator_as_list(path: &Path) -> Result<Checked> {
    let (sources, _) = inputs(path)?;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut sink = Vec::new();
    let parsed: Vec<front::Parsed> = sources.iter().filter_map(|(f, t)| front::parse(f, t, &mut sink)).collect();
    let found = project::iterator_as_list_only(&parsed);
    std::panic::set_hook(previous);
    Ok(Checked { report: Report::new("check", finish(found)), sources: sources.into_iter().collect() })
}

/// The resolver-backed diagnostics `wj build` stops on before generating code, exactly as `wj check` reports them:
/// `WJ0013` (a call with the wrong number of arguments), `WJ0019` (a private item called, imported or
/// re-exported outside its module) and `WJ0194` (a name the program reaches through an implicit module re-export).
pub fn resolver_errors(path: &Path) -> Result<Checked> {
    let (sources, _) = inputs(path)?;
    let mut sink = Vec::new();
    let parsed: Vec<front::Parsed> = sources.iter().filter_map(|(f, t)| front::parse(f, t, &mut sink)).collect();
    let refs: Vec<&front::Parsed> = parsed.iter().filter(|p| !project::is_shader(p)).collect();
    let by_file: HashMap<String, &front::Parsed> = refs.iter().map(|p| (p.file.clone(), *p)).collect();
    let mut found = Vec::new();
    project::resolve_phase(path, &refs, &by_file, &mut found);
    found.retain(|d| matches!(d.code.as_str(), "WJ0013" | "WJ0019" | "WJ0194"));
    Ok(Checked { report: Report::new("check", finish(found)), sources: sources.into_iter().collect() })
}

/// The `WJ0194` diagnostics (a name used through a module that does not re-export it, ADR-0040) of the program at
/// `path`, with their `pub use` fixes: the input of `wj fix`.
pub fn implicit_reexports(path: &Path) -> Result<Checked> {
    let (sources, _) = inputs(path)?;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut sink = Vec::new();
    let parsed: Vec<front::Parsed> = sources.iter().filter_map(|(f, t)| front::parse(f, t, &mut sink)).collect();
    let refs: Vec<&front::Parsed> = parsed.iter().filter(|p| !project::is_shader(p)).collect();
    let by_file: HashMap<String, &front::Parsed> = refs.iter().map(|p| (p.file.clone(), *p)).collect();
    let mut found = Vec::new();
    project::resolve_phase(path, &refs, &by_file, &mut found);
    std::panic::set_hook(previous);
    found.retain(|d| d.code == "WJ0194");
    Ok(Checked { report: Report::new("check", finish(found)), sources: sources.into_iter().collect() })
}

/// `local_mutability` over in-memory sources (one program); files that do not parse are left out.
pub fn local_mutability_sources(sources: &[(String, String)]) -> Vec<Diagnostic> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut sink = Vec::new();
    let parsed: Vec<front::Parsed> = sources.iter().filter_map(|(f, t)| front::parse(f, t, &mut sink)).collect();
    let refs: Vec<&front::Parsed> = parsed.iter().filter(|p| !project::is_shader(p)).collect();
    let tables = std::sync::Arc::new(crate::ir::stdlib_tables::StdlibTables::load_or_compute());
    let found = local_mut::run(&refs, tables);
    std::panic::set_hook(previous);
    finish(found)
}

/// ADR-0009: the `WJ0173` diagnostics (untrusted data reaching a command, a file path or SQL text) of the program at
/// `path`, exactly as `wj check` reports them; `wj build` and `wj run --compiled` stop on these. No cache, no other check.
pub fn taint_findings(path: &Path) -> Result<Checked> {
    let (sources, _) = inputs(path)?;
    let found = taint_findings_sources(path, &sources);
    Ok(Checked { report: Report::new("check", found), sources: sources.into_iter().collect() })
}

/// `taint_findings` over in-memory sources; files that do not parse are left out.
pub fn taint_findings_sources(root: &Path, sources: &[(String, String)]) -> Vec<Diagnostic> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut sink = Vec::new();
    let parsed: Vec<front::Parsed> = sources.iter().filter_map(|(f, t)| front::parse(f, t, &mut sink)).collect();
    let refs: Vec<&front::Parsed> = parsed.iter().filter(|p| !project::is_shader(p)).collect();
    let by_file: HashMap<String, &front::Parsed> = refs.iter().map(|p| (p.file.clone(), *p)).collect();
    let mut found = Vec::new();
    taint::run(root, &refs, &by_file, &mut found);
    std::panic::set_hook(previous);
    finish(found)
}

/// ADR-0004 migration (`wj fix`): the WJ0191 (removed `thread { }` / `<-`) and WJ0190 (un-waited task) diagnostics
/// of one file, from the front end alone.
pub fn concurrency_diagnostics(file: &str, text: &str) -> Vec<crate::diagnostics::Diagnostic> {
    let mut out = Vec::new();
    if let Some(p) = front::parse(file, text, &mut out) {
        let ix = crate::diagnostics::SourceIndex::new(file, text);
        tasks::unwaited(&p, &ix, &mut out);
    }
    out.retain(|d| d.code == crate::analyzer::task_structure::CODE || d.code == crate::parser::task_prefix::REMOVED_FORM_CODE);
    out
}

/// Check in-memory sources (used by tests and the LSP).
pub fn check_sources(root: &Path, sources: &[(String, String)]) -> Vec<Diagnostic> {
    run(root, sources)
}

fn run(root: &Path, sources: &[(String, String)]) -> Vec<Diagnostic> {
    run_with(root, sources, false).diagnostics
}

struct RunOutput {
    diagnostics: Vec<Diagnostic>,
    use_depth: usize,
    /// Digest of the non-source inputs, taken before the checks ran; `None` when not caching.
    ambient: Option<String>,
}

fn run_with(root: &Path, sources: &[(String, String)], use_cache: bool) -> RunOutput {
    let mut out = Vec::new();
    let mut parsed = Vec::new();
    for (file, text) in sources {
        if let Some(p) = front::parse(file, text, &mut out) {
            front::file_checks(&p, &mut out);
            parsed.push(p);
        }
    }
    crate::debug_env::phase("check-parse");
    let use_depth = ambient::use_depth(parsed.iter().map(|p| &p.program));
    let ambient = if use_cache { ambient::fingerprint(use_depth) } else { None };
    let mut memo = ambient.as_deref().map(|a| file_cache::FileMemo::open(root, a));
    project::run(root, &parsed, &mut out, memo.as_mut());
    if let Some(mut m) = memo {
        m.retain(&parsed.iter().map(|p| p.file.as_str()).collect());
        m.save();
    }
    RunOutput { diagnostics: finish(out), use_depth, ambient }
}

/// Deterministic order, duplicates removed.
fn finish(mut out: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let key = |d: &Diagnostic| {
        d.primary_span
            .as_ref()
            .map(|s| (s.file.clone(), s.byte_start, s.byte_end))
            .unwrap_or_default()
    };
    out.sort_by(|a, b| key(a).cmp(&key(b)).then_with(|| a.code.cmp(&b.code)));
    out.dedup_by(|a, b| a.code == b.code && key(a) == key(b));
    out
}
