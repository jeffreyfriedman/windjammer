//! `wj run --watch` / `wj test --watch`: re-run on every source change.
//!
//! The command runs as a child process (`wj` re-invoked without `--watch`) so a long-running program
//! (server, game loop) is stopped and restarted on change. Changes are detected by polling file
//! metadata every [`POLL`]: for a mid-size project (tens to hundreds of files) one poll is a few hundred
//! `stat` calls, well under a millisecond, so restart latency is poll interval + process start +
//! interpreter load (measured in docs/INTERPRETER.md). No platform notification APIs, so it behaves the
//! same on every OS and on network/VM file systems.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant, SystemTime};

pub const POLL: Duration = Duration::from_millis(20);

/// Fingerprint of the watched tree: every `.wj` / `wj.toml` / `wj.lock` / `.rs` file and its mtime + size.
pub fn snapshot(root: &Path) -> BTreeMap<PathBuf, (SystemTime, u64)> {
    let mut out = BTreeMap::new();
    fn walk(d: &Path, out: &mut BTreeMap<PathBuf, (SystemTime, u64)>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if !name.starts_with('.') && !crate::module_system::sources::SKIPPED_DIRS.contains(&name.as_str()) {
                    walk(&p, out);
                }
            } else if name.ends_with(".wj") || name == "wj.toml" || name == "wj.lock" || name.ends_with(".rs") {
                if let Ok(m) = e.metadata() {
                    out.insert(p, (m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len()));
                }
            }
        }
    }
    walk(root, &mut out);
    out
}

/// Files that differ between two snapshots (added, removed or modified).
pub fn changed(a: &BTreeMap<PathBuf, (SystemTime, u64)>, b: &BTreeMap<PathBuf, (SystemTime, u64)>) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = b.iter().filter(|(k, m)| a.get(*k) != Some(m)).map(|(k, _)| k.clone()).collect();
    v.extend(a.keys().filter(|k| !b.contains_key(*k)).cloned());
    v
}

/// Directory to watch for a run/test path: the project root (`wj.toml`) if any, else the path's directory.
pub fn watch_root(path: Option<&Path>) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    let start = path.map(|p| if p.is_absolute() { p.to_path_buf() } else { cwd.join(p) }).unwrap_or(cwd);
    if let Some(r) = crate::module_system::project::find_manifest_dir(&start) {
        return r;
    }
    if start.is_file() {
        start.parent().map(Path::to_path_buf).unwrap_or(start)
    } else {
        start
    }
}

fn spawn(args: &[String]) -> Option<Child> {
    let exe = std::env::current_exe().ok()?;
    match Command::new(exe).args(args).spawn() {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("wj watch: cannot start: {e}");
            None
        }
    }
}

/// Run `wj <args>` now and again after every change under `root`, until interrupted.
/// `max_runs` bounds the loop (tests); `None` runs forever.
pub fn watch(root: &Path, args: Vec<String>, max_runs: Option<usize>) -> i32 {
    let mut snap = snapshot(root);
    let mut child = spawn(&args);
    let mut runs = 1;
    eprintln!("[wj watch] watching {} ({} files); Ctrl-C to stop", root.display(), snap.len());
    loop {
        std::thread::sleep(POLL);
        let now = snapshot(root);
        let diff = changed(&snap, &now);
        if diff.is_empty() {
            if let Some(c) = child.as_mut() {
                if let Ok(Some(status)) = c.try_wait() {
                    eprintln!("[wj watch] exited with {}; waiting for changes", status.code().unwrap_or(-1));
                    child = None;
                    if max_runs.is_some_and(|m| runs >= m) {
                        return status.code().unwrap_or(1);
                    }
                }
            }
            continue;
        }
        let t0 = Instant::now();
        snap = now;
        if let Some(mut c) = child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        let shown: Vec<String> = diff.iter().take(3).map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string()).collect();
        eprintln!("[wj watch] changed: {}; re-running", shown.join(", "));
        child = spawn(&args);
        runs += 1;
        if crate::debug_env::enabled("WJ_WATCH_TIMING") {
            eprintln!("[wj watch] restart issued {:?} after detection", t0.elapsed());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_diff_sees_edits_additions_and_removals() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        std::fs::write(r.join("a.wj"), "1").unwrap();
        std::fs::write(r.join("notes.txt"), "x").unwrap();
        let s1 = snapshot(r);
        assert_eq!(s1.len(), 1, "only sources are watched");
        std::fs::write(r.join("a.wj"), "22").unwrap();
        std::fs::write(r.join("b.wj"), "").unwrap();
        let s2 = snapshot(r);
        let mut c = changed(&s1, &s2);
        c.sort();
        assert_eq!(c, vec![r.join("a.wj"), r.join("b.wj")]);
        std::fs::remove_file(r.join("a.wj")).unwrap();
        assert_eq!(changed(&s2, &snapshot(r)), vec![r.join("a.wj")]);
    }
}
