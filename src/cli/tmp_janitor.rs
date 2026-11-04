//! Stale scratch-directory janitor behind `wj clean --tmp` and the `wj doctor` note.
//!
//! Test harnesses and scripts create `wj-*` directories in the OS temp dir. `tests/common/scratch.rs`
//! removes them on drop, but runs that were killed (or older harness versions) leave them behind.
//! A directory is a candidate when its name starts with `wj-`, it is older than the threshold, and
//! no process id embedded in its name is alive.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Default age threshold in hours.
pub const DEFAULT_OLDER_THAN_HOURS: u64 = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stale {
    pub path: PathBuf,
    pub bytes: u64,
}

/// Numeric `-`/`_` separated segments of a directory name that could be a pid.
pub fn pid_candidates(name: &str) -> Vec<u32> {
    name.split(['-', '_'])
        .filter(|s| (2..=8).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|s| s.parse().ok())
        .collect()
}

pub fn pid_alive(pid: u32) -> bool {
    // SAFETY: signal 0 only checks existence/permission.
    let r = unsafe { libc::kill(pid as libc::pid_t, 0) };
    r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Pure decision: should `name`, last modified `age` ago, be removed?
pub fn is_candidate(name: &str, age: Duration, older_than: Duration, alive: &dyn Fn(u32) -> bool) -> bool {
    name.starts_with("wj-") && age > older_than && !pid_candidates(name).into_iter().any(alive)
}

/// Scan `temp` for removable `wj-*` directories.
pub fn scan(temp: &Path, older_than: Duration, now: SystemTime, alive: &dyn Fn(u32) -> bool) -> Vec<Stale> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(temp) else { return out };
    for e in entries.flatten() {
        let Ok(md) = e.path().symlink_metadata() else { continue };
        if !md.is_dir() {
            continue;
        }
        let age = md.modified().ok().and_then(|m| now.duration_since(m).ok()).unwrap_or_default();
        if is_candidate(&e.file_name().to_string_lossy(), age, older_than, alive) {
            out.push(Stale { bytes: crate::cargo_cache::dir_size(&e.path()), path: e.path() });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

pub fn scan_default(older_than_hours: u64) -> Vec<Stale> {
    scan(&std::env::temp_dir(), Duration::from_secs(older_than_hours * 3600), SystemTime::now(), &pid_alive)
}

/// One-line `wj doctor` note, or `None` when nothing is stale.
pub fn doctor_note() -> Option<String> {
    let found = scan_default(DEFAULT_OLDER_THAN_HOURS);
    if found.is_empty() {
        return None;
    }
    let bytes: u64 = found.iter().map(|s| s.bytes).sum();
    Some(format!(
        "note: {} stale wj-* scratch dir(s) in the temp dir ({}, older than {DEFAULT_OLDER_THAN_HOURS}h): wj clean --tmp (dry run), then wj clean --tmp --yes",
        found.len(),
        crate::cargo_cache::format_size(bytes)
    ))
}

/// `wj clean --tmp [--older-than-hours N] [--yes]`.
pub fn run(older_than_hours: u64, yes: bool) -> anyhow::Result<()> {
    let found = scan_default(older_than_hours);
    let total: u64 = found.iter().map(|s| s.bytes).sum();
    for s in &found {
        println!("  {} {} ({})", if yes { "removing" } else { "would remove" }, s.path.display(), crate::cargo_cache::format_size(s.bytes));
    }
    if !yes {
        println!("  {} stale wj-* dir(s), {}; dry run, pass --yes to remove", found.len(), crate::cargo_cache::format_size(total));
        return Ok(());
    }
    let mut removed = 0usize;
    for s in &found {
        match std::fs::remove_dir_all(&s.path) {
            Ok(()) => removed += 1,
            Err(e) => eprintln!("  warning: failed to remove {}: {e}", s.path.display()),
        }
    }
    println!("  removed {removed} dir(s), {}", crate::cargo_cache::format_size(total));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const H: Duration = Duration::from_secs(3600);

    #[test]
    fn pids_are_extracted_from_both_separators() {
        assert_eq!(pid_candidates("wj-conformance-12345-corpus"), vec![12345]);
        assert_eq!(pid_candidates("wj_vec_js_777"), vec![777]);
        assert!(pid_candidates("wj-embedded").is_empty());
    }

    #[test]
    fn decision_table() {
        let dead = |_: u32| false;
        let live = |p: u32| p == 4242;
        assert!(is_candidate("wj-x-1000-a", 7 * H, 6 * H, &dead));
        assert!(!is_candidate("wj-x-1000-a", 5 * H, 6 * H, &dead), "too young");
        assert!(!is_candidate("wj-x-4242-a", 7 * H, 6 * H, &live), "owner alive");
        assert!(!is_candidate("other-1000", 7 * H, 6 * H, &dead), "wrong prefix");
        assert!(is_candidate("wj-nopid", 7 * H, 6 * H, &dead));
    }

    #[test]
    fn scan_measures_and_skips_files() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir(t.path().join("wj-a-1000")).unwrap();
        std::fs::write(t.path().join("wj-a-1000/f"), vec![0u8; 100]).unwrap();
        std::fs::write(t.path().join("wj-file-1000"), b"x").unwrap();
        let later = SystemTime::now() + 10 * H;
        let r = scan(t.path(), 6 * H, later, &|_| false);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].bytes, 100);
        assert!(scan(t.path(), 6 * H, SystemTime::now(), &|_| false).is_empty());
    }
}
