//! RAII scratch directories for test harnesses.
//!
//! Every test that needs a `wj-*` directory in the OS temp dir goes through [`Scratch`]: the
//! directory is removed on drop, including while a panic unwinds. Set `WJ_KEEP_TMP=1` to keep it
//! for debugging (the path is printed). Include with
//! `#[path = "common/scratch.rs"] mod scratch;` (adjust the prefix for nested suites).
//!
//! Heavy shared inputs (big corpora, compiled targets) belong under [`shared_cache_dir`], not in a
//! per-process scratch directory.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct Scratch {
    path: PathBuf,
}

impl Scratch {
    /// Fresh empty directory `<tmp>/wj-<tag>-<pid>-<n>`.
    pub fn new(tag: &str) -> Scratch {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("wj-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create scratch dir");
        // Canonical (macOS: /var -> /private/var) so callers never need to shadow it with a plain PathBuf.
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        Scratch { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, p: impl AsRef<Path>) -> PathBuf {
        self.path.join(p)
    }

    pub fn keep_requested() -> bool {
        std::env::var("WJ_KEEP_TMP").is_ok_and(|v| v == "1")
    }
}

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<std::ffi::OsStr> for Scratch {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.path.as_os_str()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if Scratch::keep_requested() {
            eprintln!("WJ_KEEP_TMP=1: keeping {}", self.path.display());
            return;
        }
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Stable cache location for expensive shared inputs (inside the shared cargo cache root).
pub fn shared_cache_dir(name: &str) -> PathBuf {
    let d = windjammer::cargo_cache::cache_root().join("test-scratch").join(name);
    std::fs::create_dir_all(&d).expect("create shared cache dir");
    d
}
