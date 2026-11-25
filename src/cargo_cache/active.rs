//! Which cargo target trees a build is using right now, so automatic pruning never deletes under it.
//!
//! A build holds a shared `flock` on `<tree>/.wj-active.lock` for the length of its cargo run ([`hold`]); the lock
//! goes away with the process, even when it is killed. A prune asks for an exclusive lock without waiting
//! ([`held`]): if it cannot get one, some build holds the tree. Builds that do not go through `wj` (a plain
//! `cargo build` with the shared `CARGO_TARGET_DIR`) hold no lock, so a prune also leaves anything written within
//! [`RECENT`] alone.

use super::TargetKind;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Anything modified more recently than this may belong to a running build.
pub const RECENT: Duration = Duration::from_secs(60 * 60);

const LOCK_FILE: &str = ".wj-active.lock";

/// A tree held by a build; released when dropped (or when the process ends).
#[must_use = "the tree is held only while this value lives"]
pub struct TreeLock {
    _file: File,
}

/// Hold the target tree of `kind` for a build.
pub fn hold(kind: TargetKind) -> Option<TreeLock> {
    hold_path(&super::target_dir(kind))
}

/// Hold the target tree at `tree` for a build.
pub fn hold_path(tree: &Path) -> Option<TreeLock> {
    fs::create_dir_all(tree).ok()?;
    let mut file = File::options().create(true).append(true).open(tree.join(LOCK_FILE)).ok()?;
    // nobody holds the tree (an exclusive probe succeeds): drop the pids of earlier, finished holders
    if lock(&file, true) {
        let _ = file.set_len(0);
    }
    if !lock(&file, false) {
        return None;
    }
    // name this holder, so a refused `wj clean --all` can say whose build it is (one `pid N` line per holder)
    use std::io::Write;
    let _ = writeln!(file, "{}", std::process::id());
    Some(TreeLock { _file: file })
}

/// The pid of a live build that holds the target tree at `tree` (or one directly inside it), if any.
pub fn holder(tree: &Path) -> Option<u32> {
    if !held(tree) {
        return None;
    }
    let mut trees: Vec<PathBuf> = vec![tree.to_path_buf()];
    if let Ok(rd) = fs::read_dir(tree) {
        trees.extend(rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
    }
    let named = trees.iter().filter_map(|t| fs::read_to_string(t.join(LOCK_FILE)).ok()).flat_map(|text| {
        text.lines().filter_map(|l| l.trim().parse::<u32>().ok()).collect::<Vec<_>>()
    });
    // the holder's pid when a recorded one is still alive; pid 0 means "held, holder not named" (a build by an older wj)
    Some(named.filter(|p| pid_alive(*p)).last().unwrap_or(0))
}

/// Refuse (or, with `wait`, wait) while a build holds any of `trees`: a destructive command must never delete
/// the target tree under a running cargo. `what` names the command in the error.
pub fn require_free(trees: &[PathBuf], wait: bool) -> Result<(), String> {
    loop {
        let busy = trees.iter().find_map(|t| holder(t));
        let Some(pid) = busy else { return Ok(()) };
        if !wait {
            let who = if pid == 0 { String::new() } else { format!(" (pid {pid})") };
            return Err(format!("a build is using the shared cache{who}: finish it or run with --wait"));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(unix)]
fn pid_alive(pid: u32) -> bool {
    // SAFETY: signal 0 only probes for existence.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

#[cfg(not(unix))]
fn pid_alive(_pid: u32) -> bool {
    true
}

/// Is `tree` (or a target tree directly inside it, such as `shared` / `verify` under the cache's
/// `cargo-target`) held by a build?
pub fn held(tree: &Path) -> bool {
    let mut trees: Vec<PathBuf> = vec![tree.to_path_buf()];
    if let Ok(rd) = fs::read_dir(tree) {
        trees.extend(rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
    }
    trees.iter().any(|t| {
        let Ok(file) = File::options().append(true).open(t.join(LOCK_FILE)) else { return false };
        // an exclusive lock we can take means nobody holds the shared one; it is released when `file` closes
        !lock(&file, true)
    })
}

/// The newest modification time of a file under `path` (or of `path` itself when it is a file). Directory times
/// and the lock and marker files are left out: creating a lock file touches its directory without any build output changing.
pub fn newest_mtime(path: &Path) -> Option<SystemTime> {
    if path.is_file() {
        return fs::metadata(path).and_then(|m| m.modified()).ok();
    }
    let mut newest = None;
    if let Ok(rd) = fs::read_dir(path) {
        for e in rd.flatten() {
            let p = e.path();
            if e.file_name() == LOCK_FILE || e.file_name() == super::evict::MARKER {
                continue;
            }
            let t = if p.is_dir() { newest_mtime(&p) } else { e.metadata().and_then(|m| m.modified()).ok() };
            newest = newest.max(t);
        }
    }
    newest
}

/// Was `t` within [`RECENT`] of now?
pub fn is_recent(t: Option<SystemTime>) -> bool {
    t.and_then(|t| SystemTime::now().duration_since(t).ok()).is_some_and(|age| age < RECENT)
}

/// May a prune delete `path` (a whole tree or a directory in one)? Not while a build holds the tree it belongs to,
/// and not when anything in it is recent.
pub fn may_delete(tree: &Path, path: &Path) -> bool {
    !held(tree) && !is_recent(newest_mtime(path))
}

#[cfg(unix)]
fn lock(file: &File, exclusive_probe: bool) -> bool {
    use std::os::unix::io::AsRawFd;
    let op = if exclusive_probe { libc::LOCK_EX | libc::LOCK_NB } else { libc::LOCK_SH };
    // SAFETY: `flock` on a descriptor this function's caller owns for the call's duration.
    unsafe { libc::flock(file.as_raw_fd(), op) == 0 }
}

#[cfg(not(unix))]
fn lock(_file: &File, exclusive_probe: bool) -> bool {
    // no advisory locks: a probe never sees a holder, so only the recent-files rule protects running builds
    let _ = exclusive_probe;
    true
}
