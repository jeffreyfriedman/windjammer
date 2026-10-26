//! Shared helpers: a cwd lock (the cache keys read the working directory), cached-vs-uncached comparison.

use std::path::Path;
use std::sync::Mutex;
use windjammer::check::{check_path, CheckOptions};
use windjammer::diagnostics::Report;

static CWD: Mutex<()> = Mutex::new(());

/// Run `f` with `dir` as the working directory (serialized: the working directory is process-wide).
pub fn in_dir<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
    let _g = CWD.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir).unwrap();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::env::set_current_dir(prev).unwrap();
    r.unwrap_or_else(|e| std::panic::resume_unwind(e))
}

pub fn report(target: &Path, use_cache: bool) -> Report {
    check_path(target, &CheckOptions { use_cache }).expect("check runs").report
}

pub fn diags(r: &Report) -> String {
    serde_json::to_string_pretty(&r.diagnostics).unwrap()
}

/// Outcome of comparing one target: (whole-run cache hit on the repeat run, uncached report).
pub struct Compared {
    pub repeat_hit: bool,
    pub uncached: Report,
}

/// Cached (twice: the second run may be a whole-run hit) against uncached: identical diagnostics, identical
/// success, identical `wj run` gate decision. Panics with `context` on any difference.
pub fn compare(target: &Path, context: &str) -> Compared {
    let first = report(target, true);
    let second = report(target, true);
    let uncached = report(target, false);
    let want = diags(&uncached);
    assert_eq!(diags(&first), want, "cached (first) != uncached: {context}");
    assert_eq!(diags(&second), want, "cached (repeat) != uncached: {context}");
    assert_eq!(first.success, uncached.success, "{context}");
    assert_eq!(second.success, uncached.success, "{context}");
    for lints in [true, false] {
        let blocks = uncached.diagnostics.iter().any(|d| d.is_error() && (lints || !d.code.starts_with('W')));
        let gate = windjammer::cli::check::require_clean(target, lints);
        assert_eq!(gate.is_ok(), !blocks, "run gate (lints={lints}) differs from uncached check: {context}");
    }
    Compared { repeat_hit: second.summary.files_cached > 0, uncached }
}

pub fn write(root: &Path, files: &[(String, String)]) {
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
}

/// Deterministic xorshift generator (no dependency, reproducible failures).
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}
