//! Whole-run result cache. Sound by construction: the key covers every input of the checks
//! (all `.wj` file paths and contents) plus the compiler build, so any change anywhere
//! recomputes everything; an unchanged project is answered without lexing a single file.

use crate::diagnostics::Diagnostic;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Entry {
    format: u32,
    identity: String,
    fingerprint: u64,
    diagnostics: Vec<Diagnostic>,
}

/// Cheap build identity: version + executable size and mtime (no content hashing).
fn identity() -> String {
    let mut h = DefaultHasher::new();
    if let Some(m) = std::env::current_exe().ok().and_then(|e| std::fs::metadata(e).ok()) {
        m.len().hash(&mut h);
        m.modified().ok().hash(&mut h);
    }
    format!("{}:{:016x}", env!("CARGO_PKG_VERSION"), h.finish())
}

pub fn fingerprint(files: &[(String, String)]) -> u64 {
    let mut h = DefaultHasher::new();
    for (f, t) in files {
        f.hash(&mut h);
        t.hash(&mut h);
    }
    h.finish()
}

fn path_for(root: &Path) -> PathBuf {
    let dir = if root.is_file() { root.parent().unwrap_or(root) } else { root };
    // One file per checked path so different targets do not evict each other.
    let mut h = DefaultHasher::new();
    std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf()).hash(&mut h);
    crate::metadata::meta_cache_root(dir).join(format!("check-v1-{:08x}.json", h.finish() as u32))
}

pub fn load(root: &Path, fp: u64) -> Option<Vec<Diagnostic>> {
    let text = std::fs::read_to_string(path_for(root)).ok()?;
    let e: Entry = serde_json::from_str(&text).ok()?;
    (e.format == FORMAT && e.identity == identity() && e.fingerprint == fp).then_some(e.diagnostics)
}

pub fn store(root: &Path, fp: u64, diagnostics: &[Diagnostic]) {
    let p = path_for(root);
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let e = Entry {
        format: FORMAT,
        identity: identity(),
        fingerprint: fp,
        diagnostics: diagnostics.to_vec(),
    };
    if let Ok(s) = serde_json::to_string(&e) {
        let _ = std::fs::write(p, s);
    }
}
