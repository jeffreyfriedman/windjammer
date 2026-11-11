//! `wj run --interpret --speculate` (ADR-0033 option a, opt-in): start a compiled build of the same content in the
//! background while the interpreter runs, so the compiled verdict (`cli::verdict`) is usually known by the next run.
//!
//! Bounds (a shared machine):
//! - at most one speculative build per user at a time (a pid file in the cache root; a live build is never doubled);
//! - nothing starts when the verdict for this content and compiler is already recorded;
//! - nothing starts when the 1-minute load average is at or above the number of cores;
//! - the build runs at the lowest priority (`nice -n 19`), detached, with no terminal I/O, and offline
//!   (`CARGO_NET_OFFLINE`), so it never fetches;
//! - a project with Rust crate dependencies is not built speculatively (their build scripts would run).
//! Compiling runs none of the program's code and needs no grant. The verdict surfaces as the footer of the next
//! interpreted run (`wj: note: the compiled build of this exact program failed ...`).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Why no speculative build starts for `input`, or `None` when one may.
pub fn skip_reason(input: &Path, cache_root: &Path) -> Option<String> {
    if crate::cli::verdict::load_from(&cache_root.join("verdicts"), &crate::cli::verdict::program_key(input)?).is_some() {
        return Some("the verdict for this content is already recorded".into());
    }
    if let Some(dir) = crate::check::effects::manifest_path(input).and_then(|m| m.parent().map(Path::to_path_buf)) {
        if !crate::package::rust_surface::load(&dir).names.is_empty() {
            return Some("the project has Rust crate dependencies".into());
        }
    }
    if let Some(pid) = running(cache_root) {
        return Some(format!("speculative build {pid} is still running"));
    }
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
    if load_average().is_some_and(|l| l >= cores) {
        return Some("the machine is busy".into());
    }
    None
}

fn pid_file(cache_root: &Path) -> PathBuf {
    cache_root.join("speculative.pid")
}

/// The pid of a speculative build that is still alive.
pub fn running(cache_root: &Path) -> Option<u32> {
    let pid: u32 = std::fs::read_to_string(pid_file(cache_root)).ok()?.trim().parse().ok()?;
    alive(pid).then_some(pid)
}

fn alive(pid: u32) -> bool {
    crate::cli::tmp_janitor::pid_alive(pid)
}

fn load_average() -> Option<f64> {
    let mut l = [0f64; 3];
    // SAFETY: `getloadavg` writes at most the given number of samples into the buffer.
    let n = unsafe { libc::getloadavg(l.as_mut_ptr(), 3) };
    (n >= 1).then_some(l[0])
}

/// Start the background build of `input` (what `wj run --compiled` builds) unless a bound says no. Never waits.
pub fn start(input: &Path) -> Option<u32> {
    let root = crate::cargo_cache::cache_root();
    if skip_reason(input, &root).is_some() {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    let out = root.join("speculative").join(crate::cli::verdict::program_key(input)?.get(..16)?.to_string());
    let _ = std::fs::create_dir_all(&out);
    // `wj build` records the verdict (cli::verdict::record_cargo_json); the scratch output is removed afterwards
    let script = "nice -n 19 \"$0\" build \"$1\" --output \"$2\" --no-lint >/dev/null 2>&1; rm -rf \"$2\"";
    let child = Command::new("sh")
        .arg("-c")
        .arg(script)
        .arg(&exe)
        .arg(input)
        .arg(&out)
        .env("CARGO_NET_OFFLINE", "true")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let pid = child.id();
    let _ = std::fs::create_dir_all(&root);
    let _ = std::fs::write(pid_file(&root), pid.to_string());
    Some(pid)
}
