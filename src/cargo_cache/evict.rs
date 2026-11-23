//! Size-budget eviction for the cache trees wj itself created.
//!
//! A cargo target is a set of build units. Deleting a single file out of one (a build script's `bindgen.rs`, one
//! `.rlib`) corrupts it silently: cargo's fingerprint still says "fresh" while its outputs are gone. So eviction
//! works only at a granularity cargo can rebuild: a whole `incremental/<session>` directory, or a whole unit, which
//! is its `.fingerprint/<name>-<hash>` directory, its `build/<name>-<hash>` directory and every `deps/` file that
//! carries that hash, removed together (fingerprint first, so an interrupted eviction leaves a unit cargo rebuilds).
//!
//! Only a tree carrying the wj marker ([`MARKER`]) is ever swept. wj writes the marker into the trees it creates
//! (`cargo-target/shared`, `cargo-target/verify`); a compiler's own `debug/` / `release/` build, or any directory wj
//! did not create, has no marker and is left alone.

use super::{active, dir_size, format_size, PruneReport};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Written by wj into every cache tree it creates. A tree without it is never swept.
pub const MARKER: &str = ".wj-cache-root";

/// Mark `dir` as a cache tree wj created and may sweep.
pub fn mark(dir: &Path) -> std::io::Result<()> {
    let marker = dir.join(MARKER);
    if marker.exists() {
        return Ok(());
    }
    fs::write(marker, b"created by wj; whole cargo units in here may be evicted (see src/cargo_cache/MODULE.md)\n")
}

pub fn is_marked(dir: &Path) -> bool {
    dir.join(MARKER).is_file()
}

/// One cargo unit: everything that carries one metadata hash in one profile directory.
struct Unit {
    newest: SystemTime,
    bytes: u64,
    /// fingerprint first, then build output, then deps files
    paths: Vec<PathBuf>,
}

/// One eviction candidate: an incremental session directory or a whole unit.
struct Candidate {
    newest: SystemTime,
    bytes: u64,
    paths: Vec<PathBuf>,
}

/// The 16-hex-digit metadata hash a cargo file or directory name ends with (`name-<hash>` or `libname-<hash>.rlib`).
fn unit_hash(name: &str) -> Option<&str> {
    let stem = name.split('.').next().unwrap_or(name);
    let (_, hash) = stem.rsplit_once('-')?;
    (hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then_some(hash)
}

/// Profile directories of a cargo target: those with a `.fingerprint` directory, directly under `tree` or one
/// level down (a target-triple directory).
fn profile_dirs(tree: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(tree) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        if p.join(".fingerprint").is_dir() {
            out.push(p);
        } else if let Ok(inner) = fs::read_dir(&p) {
            out.extend(inner.flatten().map(|i| i.path()).filter(|q| q.join(".fingerprint").is_dir()));
        }
    }
    out
}

fn newest_of(path: &Path) -> SystemTime {
    active::newest_mtime(path)
        .or_else(|| fs::metadata(path).and_then(|m| m.modified()).ok())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn size_of(path: &Path) -> u64 {
    if path.is_dir() { dir_size(path) } else { fs::metadata(path).map(|m| m.len()).unwrap_or(0) }
}

fn units_of(profile: &Path, out: &mut Vec<Candidate>) {
    use std::collections::HashMap;
    // hash -> [fingerprint, build, deps] paths
    let mut by_hash: HashMap<String, [Vec<PathBuf>; 3]> = HashMap::new();
    for (slot, sub) in [".fingerprint", "build", "deps"].iter().enumerate() {
        let Ok(rd) = fs::read_dir(profile.join(sub)) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            // `build/<crate>/` holds no hash of its own when it is not a unit dir; only hashed names are units
            if let Some(h) = unit_hash(&name) {
                by_hash.entry(h.to_string()).or_default()[slot].push(e.path());
            }
        }
    }
    for (_, parts) in by_hash {
        let paths: Vec<PathBuf> = parts.into_iter().flatten().collect();
        let unit = Unit {
            newest: paths.iter().map(|p| newest_of(p)).max().unwrap_or(SystemTime::UNIX_EPOCH),
            bytes: paths.iter().map(|p| size_of(p)).sum(),
            paths,
        };
        out.push(Candidate { newest: unit.newest, bytes: unit.bytes, paths: unit.paths });
    }
}

/// Evict until `tree` fits in `budget` bytes: incremental sessions first, then whole units, least recently used
/// first. Returns bytes freed. A tree without the marker, a held tree, and anything written within the hour are left
/// alone; when the tree is over budget and eviction cannot bring it under, one warning line says so.
pub fn enforce_size_budget(tree: &Path, budget: u64, report: &mut PruneReport) -> u64 {
    let size = dir_size(tree);
    if size <= budget {
        return 0;
    }
    if !is_marked(tree) {
        report.messages.push(format!(
            "{} is over the cache budget ({}) but wj did not create it (no {} marker): not swept",
            tree.display(),
            format_size(budget),
            MARKER
        ));
        return 0;
    }
    // a running build may need any artifact of its tree, however old: leave a held tree alone
    if active::held(tree) {
        report.messages.push(format!("{} is in use by a running build: size budget not enforced", tree.display()));
        return 0;
    }
    let mut incremental: Vec<Candidate> = Vec::new();
    let mut units: Vec<Candidate> = Vec::new();
    for profile in profile_dirs(tree) {
        if let Ok(rd) = fs::read_dir(profile.join("incremental")) {
            for e in rd.flatten().filter(|e| e.path().is_dir()) {
                let p = e.path();
                incremental.push(Candidate { newest: newest_of(&p), bytes: size_of(&p), paths: vec![p] });
            }
        }
        units_of(&profile, &mut units);
    }
    incremental.sort_by_key(|c| c.newest);
    units.sort_by_key(|c| c.newest);

    let mut excess = size - budget;
    let mut freed = 0u64;
    for c in incremental.into_iter().chain(units) {
        if excess == 0 {
            break;
        }
        // written within the hour: possibly by a build that holds no lock
        if active::is_recent(Some(c.newest)) {
            continue;
        }
        let mut ok = true;
        for p in &c.paths {
            let r = if p.is_dir() { fs::remove_dir_all(p) } else { fs::remove_file(p) };
            ok &= r.is_ok();
        }
        if ok {
            freed += c.bytes;
            excess = excess.saturating_sub(c.bytes);
        }
    }
    if freed > 0 {
        report.messages.push(format!(
            "Cache over budget ({}): evicted whole cargo units, freed {}",
            format_size(budget),
            format_size(freed)
        ));
    }
    if excess > 0 {
        report.messages.push(format!(
            "{} is still {} over the cache budget ({}): nothing more can be evicted safely (recent or non-unit data)",
            tree.display(),
            format_size(excess),
            format_size(budget)
        ));
    }
    freed
}
