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
        reap_dead_scratch();
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

/// Run one `#[ignore]`d test of this very test binary in a child process with `envs` set only there, and fail with its
/// output unless it passed. For tests that must point a process-global setting (`WJ_CARGO_TARGET_ROOT`) at a private
/// directory: `std::env::set_var` in the shared test process would send every concurrent and later build of every
/// other test there too (one such root grew to 3.4 GB of real cargo targets and outlived its scratch directory).
/// `name` is the test function's name inside the calling module (`module_path!()` supplies the module).
pub fn run_ignored_test_in_child(module: &str, name: &str, envs: &[(&str, &std::ffi::OsStr)]) {
    // `all::cargo_cache_active_test` in the consolidated binary, the crate name alone in a standalone one
    let filter = module.split_once("::").map_or(name.to_string(), |(_, m)| format!("{m}::{name}"));
    let mut c = std::process::Command::new(std::env::current_exe().expect("test binary"));
    c.args([filter.as_str(), "--exact", "--ignored", "--nocapture", "--test-threads=1"]).env(CHILD_MARKER, "1");
    for (k, v) in envs {
        c.env(k, v);
    }
    let out = c.output().expect("run test child");
    let text = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success() && text.contains("1 passed"), "child test {filter} failed:\n{text}");
}

/// Set in the child that [`run_ignored_test_in_child`] starts; the `#[ignore]`d body returns at once without it, so a
/// plain `--ignored` run never changes the shared process's settings.
pub const CHILD_MARKER: &str = "WJ_TEST_CHILD";

pub fn in_test_child() -> bool {
    std::env::var_os(CHILD_MARKER).is_some()
}

/// Is a process with this id running? (`/proc` where it exists, else `kill -0`.) A pid we cannot tell about counts as alive.
fn process_alive(pid: u32) -> bool {
    if Path::new("/proc/self").exists() {
        return Path::new(&format!("/proc/{pid}")).exists();
    }
    std::process::Command::new("kill").args(["-0", &pid.to_string()]).stderr(std::process::Stdio::null()).status().map_or(true, |s| s.success())
}

/// Remove the scratch directories that killed test processes left behind: a test run that is killed (a timeout, a
/// `kill -9`, a machine restart) never runs `Drop`, and one leaked directory can hold gigabytes (the cache tests build
/// real cargo targets in theirs). Scratch names carry the owner's process id (`wj-<tag>-<pid>-<n>`), so a directory
/// whose process is gone, and which nothing has touched for ten minutes (a recycled pid is then not mistaken for the
/// owner), is stale. Runs once per process, on the first [`Scratch::new`]; `WJ_KEEP_TMP=1` keeps everything.
fn reap_dead_scratch() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if Scratch::keep_requested() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else { return };
        let idle = std::time::Duration::from_secs(10 * 60);
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let Some(rest) = name.strip_prefix("wj-") else { continue };
            let mut parts = rest.rsplitn(3, '-');
            let (Some(n), Some(pid), Some(_tag)) = (parts.next(), parts.next(), parts.next()) else { continue };
            let (Ok(_), Ok(pid)) = (n.parse::<u32>(), pid.parse::<u32>()) else { continue };
            let untouched = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age > idle);
            if pid != std::process::id() && untouched && !process_alive(pid) {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    });
}
