//! Check inputs that are not `.wj` sources, for the cache keys: the cached `.wj.meta` metadata the numeric inference
//! reads relative to the working directory (`<cwd>/.wj-cache/<module>/<x>.wj.meta`, see
//! `type_inference/float_inference/load_imported_metadata.rs`). Candidate paths come from top-level `use` paths,
//! so they lie within `use_depth` directory levels of the working directory.

use crate::parser::Item;
use sha2::{Digest, Sha256};
use std::path::Path;

/// Directory entries a fingerprint may visit; beyond it the run is not cached at all.
const WALK_BUDGET: usize = 20_000;

/// What the sidecar inference can read for a program: the deepest `.wj.meta` candidate (directory levels below the
/// working directory, file included; one level of slack) and the path segments of its top-level `use`s. A candidate
/// sidecar is `<segment>.wj.meta` or lies in a directory named by a segment, so nothing else is looked at.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UseScope {
    depth: usize,
    names: Vec<String>,
}

/// The [`UseScope`] of `programs`.
pub fn use_depth<'a>(programs: impl IntoIterator<Item = &'a crate::parser::Program<'static>>) -> UseScope {
    let mut depth = 0;
    let mut names = std::collections::BTreeSet::new();
    for p in programs {
        for item in &p.items {
            if let Item::Use { path, .. } = item {
                let levels: usize = path.iter().map(|s| 1 + s.matches(['/', '\\']).count()).sum();
                depth = depth.max(levels + 2);
                for seg in path.iter().flat_map(|s| s.split(['/', '\\'])).filter(|s| !s.is_empty() && !s.starts_with('.')) {
                    names.insert(seg.to_string());
                }
            }
        }
    }
    UseScope { depth, names: names.into_iter().collect() }
}

thread_local! {
    static CWD_OVERRIDE: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// The working directory the check caches key on: the process's, unless this thread runs [`with_cwd`]. Tests use the
/// override instead of changing the process-wide directory (which pulls it from under every concurrent test).
pub fn cwd() -> std::io::Result<std::path::PathBuf> {
    match CWD_OVERRIDE.with(|c| c.borrow().clone()) {
        Some(d) => Ok(d),
        None => std::env::current_dir(),
    }
}

/// Run `f` with [`cwd`] answering `dir` on this thread (restored afterwards, also on panic). Never touches the process.
pub fn with_cwd<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<std::path::PathBuf>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CWD_OVERRIDE.with(|c| *c.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(CWD_OVERRIDE.with(|c| c.borrow_mut().replace(dir.to_path_buf())));
    f()
}

/// Digest of every `*.wj.meta` sidecar within `depth` levels of the working directory and of its `.wj-cache/`,
/// independent of where the working directory is: sidecars are named relative to it, so the same sidecar contents
/// give the same digest in any checkout. It keys the whole-run cache, the per-project memo and the shared analysis
/// cache alike. `None` when the working directory is unknown, the walk exceeds its budget, or a sidecar lies outside
/// the working directory (a path that cannot be named portably): the caller must not cache.
pub fn fingerprint(scope: &UseScope) -> Option<String> {
    fingerprint_in(&cwd().ok()?, scope)
}

/// [`fingerprint`] for the working directory `cwd`.
fn fingerprint_in(cwd: &Path, scope: &UseScope) -> Option<String> {
    let depth = scope.depth;
    let names = &scope.names[..];
    let mut files = Vec::new();
    let mut budget = WALK_BUDGET;
    // Only cache locations are read (colocated `.wj.meta` sidecars are ignored by the inference).
    walk(&cwd.join(".wj-cache"), names, depth + 1, &mut budget, &mut files)?;
    // `wj check` holds no `CompilerConfig`: the process's shared cache root holds manifest-less entries
    let cache_root = crate::compiler_config::shared_cache_root();
    walk(&crate::metadata::meta_cache_root(cwd, &cache_root), names, depth + 1, &mut budget, &mut files)?;
    // a manifest-less program's entries are per module file: the ones its `use` names count like the tree's
    for m in crate::metadata::peer_meta_entries(cwd, &cache_root) {
        let stem = m.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".wj.meta")).unwrap_or("");
        let module = if stem == "mod" { m.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("") } else { stem };
        if names.iter().any(|n| n == module) {
            files.push(m);
        }
    }
    files.sort();
    files.dedup();
    let mut h = Sha256::new();
    for f in &files {
        let rel = f.strip_prefix(cwd).ok()?;
        part(&mut h, rel.to_string_lossy().replace('\\', "/").as_bytes());
        crate::metadata::scan_stats::ambient_file();
        part(&mut h, &std::fs::read(f).unwrap_or_default());
    }
    Some(format!("{:x}", h.finalize()))
}

/// Length-prefixed, so adjacent parts cannot run together.
pub fn part(h: &mut Sha256, bytes: &[u8]) {
    h.update((bytes.len() as u64).to_le_bytes());
    h.update(bytes);
}

/// The candidate sidecars under `dir`: for each name, `<name>.wj.meta` and, inside the directory `<name>`, every
/// sidecar and (recursively) the sub-directories that are themselves named. Entries of `dir` no name points at are
/// neither listed nor read, so unrelated files cost nothing.
fn walk(dir: &Path, names: &[String], depth: usize, budget: &mut usize, out: &mut Vec<std::path::PathBuf>) -> Option<()> {
    if depth == 0 {
        return Some(());
    }
    for n in names {
        *budget = budget.checked_sub(1)?;
        let file = dir.join(format!("{n}.wj.meta"));
        if file.is_file() {
            out.push(file);
        }
        let sub = dir.join(n);
        if sub.is_dir() {
            list_module_dir(&sub, names, depth - 1, budget, out)?;
        }
    }
    Some(())
}

fn list_module_dir(dir: &Path, names: &[String], depth: usize, budget: &mut usize, out: &mut Vec<std::path::PathBuf>) -> Option<()> {
    if depth == 0 {
        return Some(());
    }
    crate::metadata::scan_stats::dir_listed();
    let Ok(entries) = std::fs::read_dir(dir) else { return Some(()) };
    for e in entries.flatten() {
        *budget = budget.checked_sub(1)?;
        let p = e.path();
        if p.is_dir() {
            let name = e.file_name();
            if names.iter().any(|n| name.to_string_lossy() == n.as_str()) {
                list_module_dir(&p, names, depth - 1, budget, out)?;
            }
        } else if e.file_name().to_string_lossy().ends_with(".wj.meta") {
            out.push(p);
        }
    }
    Some(())
}

#[cfg(test)]
mod tests;
