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

const FORMAT: u32 = 2;

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
    let mut h = DefaultHasher::new();
    if let Ok(exe) = std::env::current_exe() {
        exe.hash(&mut h);
        if let Ok(m) = std::fs::metadata(&exe) {
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

/// `<project>/.wj-cache/check-<kind>-v2-<hash of the checked path>.json` (one file per checked path).
pub(super) fn path_for(root: &Path, kind: &str) -> PathBuf {
    let dir = if root.is_file() { root.parent().unwrap_or(root) } else { root };
    let mut h = DefaultHasher::new();
    std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf()).hash(&mut h);
    crate::metadata::meta_cache_root(dir).join(format!("check-{kind}-v{FORMAT}-{:08x}.json", h.finish() as u32))
}

/// Write through a temporary file and a rename, so a concurrent reader never sees half a file.
pub(super) fn write_atomically(path: &Path, text: &str) {
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    if std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

pub fn load(root: &Path, fp: &str) -> Option<Vec<Diagnostic>> {
    let text = std::fs::read_to_string(path_for(root, "run")).ok()?;
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
        write_atomically(&p, &s);
    }
}
