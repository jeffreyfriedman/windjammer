//! Directories of small content-addressed entries (`<key>.json`, key = 64 hex digits) that several checkouts and
//! processes share: the analysis cache (`<cache root>/analysis`) and the compiled verdicts (`verdicts`).
//!
//! Writers hold the directory's shared build lock ([`active::hold_path`]) while they write and evict, so
//! `wj cache prune` (which takes the exclusive probe, [`active::held`]) never deletes under a writer and skips a
//! directory somebody is writing to. An entry is written to a unique temporary file and renamed into place, so a
//! reader sees a whole entry or none; entry names are keys, never paths.

use super::active;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Entries kept in the analysis directory (newest win).
pub const ANALYSIS_MAX_ENTRIES: usize = 20_000;
/// Entries older than this are dropped by `wj cache prune`.
pub const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// A temporary file older than this belongs to a writer that died.
const STALE_TMP: Duration = Duration::from_secs(60 * 60);

pub fn analysis_dir() -> PathBuf {
    super::cache_root().join("analysis")
}

/// The scanned runtime signature layer (`<cache root>/registry`, a handful of entries: one per compiler build).
pub fn registry_dir() -> PathBuf {
    super::cache_root().join("registry")
}

pub fn verdicts_dir() -> PathBuf {
    super::cache_root().join("verdicts")
}

/// `<key>.json` for a hex-digest key; anything else never names a file.
pub fn file_in(dir: &Path, key: &str) -> Option<PathBuf> {
    (key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit())).then(|| dir.join(format!("{key}.json")))
}

pub fn read(dir: &Path, key: &str) -> Option<String> {
    fs::read_to_string(file_in(dir, key)?).ok()
}

/// Write `text` under `key` and keep the newest `max` entries. Best effort: a cache that cannot be written is no error.
pub fn write(dir: &Path, key: &str, text: &str, max: usize) {
    let Some(path) = file_in(dir, key) else { return };
    // held until the end of the function: a prune that asks for the directory meanwhile is told it is in use
    let _guard = active::hold_path(dir);
    if _guard.is_none() {
        return;
    }
    let tmp = dir.join(format!(".{key}.tmp{}-{:?}-{}", std::process::id(), std::thread::current().id(), SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_nanos())).replace(['(', ')'], ""));
    if fs::write(&tmp, text).is_ok() && fs::rename(&tmp, &path).is_err() {
        let _ = fs::remove_file(&tmp);
    }
    evict(dir, key, max);
}

fn entries(dir: &Path) -> Vec<(SystemTime, PathBuf)> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| Some((fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect()
}

/// Delete all but the newest `max` entries. Listing a large directory costs, so a big one is only scanned by
/// the writes whose key starts with `0` (one in sixteen); `wj cache prune` enforces the bound exactly.
fn evict(dir: &Path, key: &str, max: usize) {
    if max > 1024 && !key.starts_with('0') {
        return;
    }
    let mut all = entries(dir);
    if all.len() <= max {
        return;
    }
    all.sort();
    for (_, p) in &all[..all.len() - max] {
        let _ = fs::remove_file(p);
    }
}

/// `wj cache prune` for one entry directory: drop entries past [`MAX_AGE`] (or past `max`) and dead writers'
/// temporary files. A directory a writer holds is left alone (and said so) unless `force`.
pub fn prune(dir: &Path, max: usize, force: bool, report: &mut super::PruneReport) -> u64 {
    if !dir.is_dir() {
        return 0;
    }
    if !force && active::held(dir) {
        report.messages.push(format!("{} is being written: not pruned", dir.display()));
        return 0;
    }
    let now = SystemTime::now();
    let mut freed = 0;
    let mut all = entries(dir);
    all.sort();
    let over = all.len().saturating_sub(max);
    for (i, (t, p)) in all.iter().enumerate() {
        let old = now.duration_since(*t).is_ok_and(|a| a > MAX_AGE);
        if old || i < over {
            freed += fs::metadata(p).map_or(0, |m| m.len());
            if fs::remove_file(p).is_ok() {
                report.removed_paths.push(p.clone());
            }
        }
    }
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let tmp = p.file_name().is_some_and(|n| n.to_string_lossy().contains(".tmp"));
            let stale = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| now.duration_since(t).ok()).is_some_and(|a| a > STALE_TMP);
            if tmp && stale {
                freed += e.metadata().map_or(0, |m| m.len());
                let _ = fs::remove_file(&p);
            }
        }
    }
    freed
}

/// Entries kept in the registry directory (newest win).
pub const REGISTRY_MAX_ENTRIES: usize = 16;

/// The scanned runtime layer as a store for `analyzer::LayerStore`.
pub struct RegistryStore {
    pub dir: PathBuf,
}

impl RegistryStore {
    pub fn get(&self, key: &str) -> Option<String> {
        read(&self.dir, key)
    }
    pub fn put(&self, key: &str, text: &str) {
        let _ = fs::create_dir_all(&self.dir);
        write(&self.dir, key, text, REGISTRY_MAX_ENTRIES);
    }
}

/// The analysis cache as a store for `check::shared_cache`.
pub struct AnalysisStore {
    pub dir: PathBuf,
}

impl AnalysisStore {
    pub fn get(&self, key: &str) -> Option<String> {
        read(&self.dir, key)
    }
    pub fn put(&self, key: &str, text: &str) {
        write(&self.dir, key, text, ANALYSIS_MAX_ENTRIES);
    }
}

