//! Per-file memo of the analysis phase (analyzer, numeric inference, IR solvers: `project::analysis_phase`).
//!
//! That phase reads one parsed file and the stdlib only (it builds a fresh `SignatureRegistry::new()`, never the
//! project's signatures), so its complete input set is: the file's path and text, the compiler build, and the
//! ambient `.wj.meta` sidecars (`ambient.rs`). Entries are keyed on the path plus a SHA-256 of the text; the whole
//! memo is dropped when the build or the ambient digest differs. Whole-program phases (resolution, effects,
//! semantic index) always run; whether a file's analysis is reported still depends on their fresh results.

use super::ambient::part;
use crate::diagnostics::Diagnostic;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize, Default)]
struct Stored {
    format: u32,
    header: String,
    entries: BTreeMap<String, (String, Vec<Diagnostic>)>,
}

pub struct FileMemo {
    path: PathBuf,
    stored: Stored,
    dirty: bool,
}

pub fn text_hash(text: &str) -> String {
    let mut h = Sha256::new();
    part(&mut h, text.as_bytes());
    format!("{:x}", h.finalize())
}

impl FileMemo {
    /// The memo for checks of `root`; `ambient` is the digest from `ambient::fingerprint`.
    pub fn open(root: &Path, ambient: &str) -> Self {
        let path = super::cache::path_for(root, "files");
        let header = format!("{}|{ambient}", super::cache::identity());
        let stored = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<Stored>(&t).ok())
            .filter(|s| s.format == FORMAT && s.header == header)
            .unwrap_or(Stored { format: FORMAT, header, entries: BTreeMap::new() });
        Self { path, stored, dirty: false }
    }

    pub fn get(&self, file: &str, text: &str) -> Option<Vec<Diagnostic>> {
        let (hash, diags) = self.stored.entries.get(file)?;
        (*hash == text_hash(text)).then(|| diags.clone())
    }

    pub fn put(&mut self, file: &str, text: &str, diags: &[Diagnostic]) {
        self.stored.entries.insert(file.to_string(), (text_hash(text), diags.to_vec()));
        self.dirty = true;
    }

    /// Forget files that are no longer part of the checked program.
    pub fn retain(&mut self, files: &std::collections::HashSet<&str>) {
        let before = self.stored.entries.len();
        self.stored.entries.retain(|f, _| files.contains(f.as_str()));
        self.dirty |= self.stored.entries.len() != before;
    }

    pub fn save(self) {
        if !self.dirty {
            return;
        }
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string(&self.stored) {
            super::cache::write_atomically(&self.path, &s);
        }
    }
}
