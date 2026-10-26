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
mod front;
pub mod locate;
mod project;
mod semantic;
pub mod watch;

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
    let paths = files::collect(path)?;
    let mut sources: Vec<(String, String)> = Vec::with_capacity(paths.len());
    for p in &paths {
        sources.push((p.display().to_string(), std::fs::read_to_string(p)?));
    }
    // inputs that change what the effect pass reports: the manifest, the lock file and dependency sources
    let mut keyed = sources.clone();
    if let Some(manifest) = effects::manifest_path(path) {
        let dir = manifest.parent().unwrap_or(path).to_path_buf();
        for extra in [manifest.clone(), dir.join(crate::ir::effect_facts::lock::LOCK_FILE)].into_iter().chain(dep_sources::input_paths(&dir)) {
            if let Ok(t) = std::fs::read_to_string(&extra) {
                keyed.push((extra.display().to_string(), t));
            }
        }
    }
    let fp = cache::fingerprint(&keyed);
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
