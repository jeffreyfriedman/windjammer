//! Whole-run result cache. Sound by construction: the key covers every input of the checks (all `.wj` file
//! paths and contents, the manifest, lock file and dependency sources, the `.wj.meta` sidecars the numeric
//! inference may read, see `ambient.rs`) plus the compiler build, so any change anywhere recomputes the run; an
//! unchanged project is answered without lexing a single file. Per-file reuse after an edit is `file_cache.rs`.

use super::ambient::{self, part};
use crate::diagnostics::Diagnostic;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

const FORMAT: u32 = 3;

#[derive(Serialize, Deserialize)]
struct Entry {
    format: u32,
    identity: String,
    fingerprint: String,
    /// `ambient::use_depth` of the cached sources (identical sources give the identical depth).
    use_depth: usize,
    ambient: String,
    diagnostics: Vec<Diagnostic>,
}

/// Cheap build identity: version + executable path, size and mtime (no content hashing).
pub(super) fn identity() -> String {
    std::env::current_exe().map_or_else(|_| identity_of(None), |exe| identity_of(Some(&exe)))
}

/// The identity of the compiler executable at `exe` (this crate's version).
pub(super) fn identity_of(exe: Option<&Path>) -> String {
    let mut h = DefaultHasher::new();
    if let Some(exe) = exe {
        exe.to_path_buf().hash(&mut h);
        if let Ok(m) = std::fs::metadata(exe) {
            m.len().hash(&mut h);
            m.modified().ok().hash(&mut h);
        }
    }
    format!("{}:{:016x}", env!("CARGO_PKG_VERSION"), h.finish())
}

/// SHA-256 over every (path, content) input, length-prefixed.
pub fn fingerprint(files: &[(String, String)]) -> String {
    let mut h = Sha256::new();
    for (f, t) in files {
        part(&mut h, f.as_bytes());
        part(&mut h, t.as_bytes());
    }
    format!("{:x}", h.finalize())
}

/// The directory cache contents are relative to: the project root (`wj.toml`/`Cargo.toml`), else the checked directory.
fn base_of(root: &Path) -> PathBuf {
    let dir = if root.is_file() { root.parent().unwrap_or(root) } else { root };
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    crate::metadata::find_project_root(dir).map_or_else(|| canon(dir), |p| canon(&p))
}

/// Placeholders that stand for the project root inside stored cache text, one per spelling a path may use: as the
/// checked paths spell it, absolute, and canonical (they differ through symlinks, e.g. macOS `/var` -> `/private/var`).
const MARK_GIVEN: &str = "{PROJECT_ROOT}";
const MARK_ABS: &str = "{PROJECT_ROOT_ABS}";
const MARK_REAL: &str = "{PROJECT_ROOT_REAL}";

/// The JSON-escaped spelling of a path, without the quotes.
fn json_spelling(p: &Path) -> Option<String> {
    let j = serde_json::to_string(&p.to_string_lossy()).ok()?;
    Some(j.trim_matches('"').to_string()).filter(|s| !s.is_empty() && s != "/" && s != ".")
}

/// (placeholder, spelling) pairs for the project of `root`, in priority order.
fn root_spellings(root: &Path) -> Vec<(&'static str, String)> {
    let dir = if root.is_file() { root.parent().unwrap_or(root) } else { root };
    let given = crate::metadata::find_project_root(dir);
    let mut v: Vec<(&'static str, String)> = Vec::new();
    if let Some(p) = &given {
        v.extend(json_spelling(p).map(|s| (MARK_GIVEN, s)));
        v.extend(std::path::absolute(p).ok().and_then(|a| json_spelling(&a)).map(|s| (MARK_ABS, s)));
    }
    v.extend(json_spelling(&base_of(root)).map(|s| (MARK_REAL, s)));
    v
}

/// Serialized cache text with the project root replaced by placeholders: a stored cache holds no absolute
/// location, so a copied or moved checkout carries none and a stale entry from another location never matches.
pub(super) fn mask_root(root: &Path, text: &str) -> String {
    let mut sp = root_spellings(root);
    // a spelling that equals an earlier one is covered by it; the longest spelling is replaced first
    let mut seen = std::collections::HashSet::new();
    sp.retain(|(_, s)| seen.insert(s.clone()));
    sp.sort_by_key(|(_, s)| std::cmp::Reverse(s.len()));
    sp.iter().fold(text.to_string(), |t, (mark, s)| t.replace(s.as_str(), mark))
}

/// Inverse of [`mask_root`] for the project at `root`: each placeholder gets back the spelling it stood for.
pub(super) fn unmask_root(root: &Path, text: &str) -> String {
    let sp = root_spellings(root);
    let mut t = text.to_string();
    for mark in [MARK_REAL, MARK_ABS, MARK_GIVEN] {
        // a spelling missing here (no distinct absolute form) falls back to the first one known
        if let Some((_, s)) = sp.iter().find(|(m, _)| *m == mark).or_else(|| sp.first()) {
            t = t.replace(mark, s);
        }
    }
    t
}

/// `(path, text)` inputs with the project root stripped from each path, so the fingerprint is location-free.
pub(super) fn relative_inputs(root: &Path, files: &[(String, String)]) -> Vec<(String, String)> {
    files.iter().map(|(f, t)| (mask_root(root, f), t.clone())).collect()
}

/// `<project>/.wj-cache/check-<kind>-v2-<hash of the checked path relative to the project>.json` (one file per checked path).
pub(super) fn path_for(root: &Path, kind: &str) -> PathBuf {
    let dir = if root.is_file() { root.parent().unwrap_or(root) } else { root };
    let mut h = DefaultHasher::new();
    let canon = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let base = base_of(root);
    canon.strip_prefix(&base).unwrap_or(&canon).hash(&mut h);
    crate::metadata::meta_cache_root(dir).join(format!("check-{kind}-v{FORMAT}-{:08x}.json", h.finish() as u32))
}

/// Write through a temporary file and a rename, so a concurrent reader never sees half a file.
pub(super) fn write_atomically(path: &Path, text: &str) {
    let tmp = path.with_extension(format!("tmp{}", super::shared_cache::unique_suffix()));
    if std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

pub fn load(root: &Path, fp: &str) -> Option<Vec<Diagnostic>> {
    let text = unmask_root(root, &std::fs::read_to_string(path_for(root, "run")).ok()?);
    let e: Entry = serde_json::from_str(&text).ok()?;
    let hit = e.format == FORMAT && e.identity == identity() && e.fingerprint == fp;
    (hit && ambient::fingerprint(e.use_depth).as_deref() == Some(e.ambient.as_str())).then_some(e.diagnostics)
}

/// `ambient` is the digest taken before the checks ran (`None`: not cacheable).
pub fn store(root: &Path, fp: &str, use_depth: usize, ambient: Option<&str>, diagnostics: &[Diagnostic]) {
    let Some(ambient) = ambient else { return };
    let p = path_for(root, "run");
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let e = Entry {
        format: FORMAT,
        identity: identity(),
        fingerprint: fp.to_string(),
        use_depth,
        ambient: ambient.to_string(),
        diagnostics: diagnostics.to_vec(),
    };
    if let Ok(s) = serde_json::to_string(&e) {
        write_atomically(&p, &mask_root(root, &s));
    }
}
