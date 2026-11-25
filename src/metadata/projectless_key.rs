//! The cache key of a manifest-less program: the hash of its MODULE CLOSURE.
//!
//! The closure is the entry file plus the files its `use` / `mod` declarations resolve to, found by following the
//! imports (a textual scan; a candidate is only `stat`ed). No directory is ever listed to build it, so the cost
//! depends on the program, never on what surrounds it (the cwd, its siblings, a parent holding other repos).
//! The key changes when any closure file changes and ignores every file outside the closure.
//!
//! Dir-based callers (`meta_cache_root(dir)`) hold no entry file; they use the process's program entry
//! ([`set_program_entry`], registered by the CLI) when it lies under that directory, else the directory's own
//! `main.wj` / `lib.wj` / `mod.wj`, else its direct `.wj` children.

use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

const MAX_CLOSURE_FILES: usize = 4096;

static ENTRY: Mutex<Option<PathBuf>> = Mutex::new(None);
type Stamp = Vec<(PathBuf, Option<SystemTime>, u64)>;
static MEMO: Mutex<Option<HashMap<(PathBuf, Option<PathBuf>), (Stamp, String)>>> = Mutex::new(None);

/// Register the program a process was started on (a file or a directory); dir-based key lookups under it use it.
pub fn set_program_entry(entry: &Path) {
    *ENTRY.lock().unwrap() = Some(canon(entry));
}

fn canon(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

/// The cache key of the manifest-less program started at `entry` (a file, or a directory): its module closure's content.
pub fn program_key(entry: &Path) -> String {
    tree_key(entry)
}

/// The key for the program `start` (a file, or a directory), memoised for the process while
/// the closure files are unchanged (size and modification time).
pub(crate) fn tree_key(start: &Path) -> String {
    let start = canon(start);
    let entry = ENTRY.lock().unwrap().clone();
    let memo_key = (start.clone(), entry.clone());
    if let Some((stamp, key)) = MEMO.lock().unwrap().as_ref().and_then(|m| m.get(&memo_key)) {
        if stamp.iter().all(|(p, mt, len)| stamp_of(p) == (*mt, *len)) {
            return key.clone();
        }
    }
    let (files, key) = tree_key_uncached(&start, entry.as_deref());
    let stamp: Stamp = files.iter().map(|p| { let (m, l) = stamp_of(p); (p.clone(), m, l) }).collect();
    MEMO.lock().unwrap().get_or_insert_with(HashMap::new).insert(memo_key, (stamp, key.clone()));
    key
}

fn stamp_of(p: &Path) -> (Option<SystemTime>, u64) {
    std::fs::metadata(p).map(|m| (m.modified().ok(), m.len())).unwrap_or((None, 0))
}

/// The key of `start`'s closure, recomputed (no memo), and the closure files.
pub(crate) fn tree_key_uncached(start: &Path, entry: Option<&Path>) -> (Vec<PathBuf>, String) {
    let (files, base) = tree_files(start, entry);
    let key = hash_files(&files, &base);
    (files, key)
}

/// The closure files of the program `start` (a file, or a directory) and the directory they are named relative to.
/// One program, one closure: a module file of the registered program and the directory holding it share the entry's.
fn tree_files(start: &Path, entry: Option<&Path>) -> (Vec<PathBuf>, PathBuf) {
    let effective = match entry {
        Some(e) if e.starts_with(start) || start.starts_with(if e.is_dir() { e } else { e.parent().unwrap_or(e) }) => e,
        _ => start,
    };
    let seeds = seeds_of(effective);
    let base = seeds.first().and_then(|s| s.parent()).map(Path::to_path_buf).unwrap_or_else(|| effective.to_path_buf());
    (closure(&seeds, &base, true), base)
}

/// The module files of the program `start` (a file, or a directory) that can hold peer metadata: its closure (every
/// `use` and `mod` edge), found by `stat`s without listing a directory.
pub(crate) fn peer_sources(start: &Path) -> Vec<PathBuf> {
    let start = canon(start);
    let entry = ENTRY.lock().unwrap().clone();
    tree_files(&start, entry.as_deref()).0
}

/// The cache key of ONE module file's metadata entry: the hash of the files it depends on (itself, the modules its
/// `use` declarations reach and, for a file that declares them, its `mod` children), named relative to the tree root.
/// Editing the file or something it depends on gives a new key; editing a sibling it does not depend on, or the
/// program's entry file, leaves it as it was, so a regenerated file still finds the entries its peers wrote.
pub(crate) fn file_key(file: &Path, tree_root: &Path) -> String {
    let file = canon(file);
    let files = closure(std::slice::from_ref(&file), &canon(tree_root), false);
    hash_files(&files, &canon(tree_root))
}

fn hash_files(files: &[PathBuf], base: &Path) -> String {
    let mut rows: Vec<(String, [u8; 32])> = files
        .iter()
        .filter_map(|p| {
            let info = info_of(p)?;
            let rel = p.strip_prefix(base).map(|r| r.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>().join("/"));
            Some((rel.unwrap_or_else(|_| p.to_string_lossy().to_string()), info.hash))
        })
        .collect();
    rows.sort();
    let mut h = Sha256::new();
    for (rel, text_hash) in &rows {
        h.update(rel.as_bytes());
        h.update([0]);
        h.update(text_hash);
    }
    h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// What one closure file contributes, read once per process while the file is unchanged (size and modification time).
struct FileInfo {
    hash: [u8; 32],
    uses: BTreeSet<String>,
    mods: BTreeSet<String>,
}

type InfoMemo = HashMap<PathBuf, (Option<SystemTime>, u64, Arc<FileInfo>)>;
static INFOS: Mutex<Option<InfoMemo>> = Mutex::new(None);

fn info_of(p: &Path) -> Option<Arc<FileInfo>> {
    let (m, l) = stamp_of(p);
    if let Some((mt, len, info)) = INFOS.lock().unwrap().as_ref().and_then(|c| c.get(p)) {
        if *mt == m && *len == l {
            return Some(info.clone());
        }
    }
    crate::metadata::scan_stats::closure_file();
    let bytes = std::fs::read(p).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let (uses, mods) = imported_names(&text);
    let info = Arc::new(FileInfo { hash: Sha256::digest(&bytes).into(), uses, mods });
    INFOS.lock().unwrap().get_or_insert_with(HashMap::new).insert(p.to_path_buf(), (m, l, info.clone()));
    Some(info)
}

fn seeds_of(start: &Path) -> Vec<PathBuf> {
    if !start.is_dir() {
        return vec![start.to_path_buf()];
    }
    let named: Vec<PathBuf> = ["main.wj", "lib.wj", "mod.wj"].iter().map(|n| start.join(n)).filter(|p| p.is_file()).collect();
    if !named.is_empty() {
        return named;
    }
    crate::metadata::scan_stats::dir_listed();
    let mut direct: Vec<PathBuf> = std::fs::read_dir(start)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wj") && p.is_file()).collect())
        .unwrap_or_default();
    direct.sort();
    direct
}

/// The files reachable from `seeds` by `use` / `mod` declarations, sorted. `mods_everywhere`: every file's `mod`
/// children count; otherwise only the seeds' (a file reached through a `use` of its parent module does not depend on
/// that module's other children).
fn closure(seeds: &[PathBuf], crate_root: &Path, mods_everywhere: bool) -> Vec<PathBuf> {
    let mut seen: BTreeSet<PathBuf> = seeds.iter().cloned().collect();
    let mut queue: Vec<PathBuf> = seeds.to_vec();
    while let Some(file) = queue.pop() {
        if seen.len() > MAX_CLOSURE_FILES {
            break;
        }
        let Some(info) = info_of(&file) else { continue };
        let dir = file.parent().unwrap_or(crate_root);
        let child_dir = if file.file_stem().is_some_and(|s| s == "mod") { dir.to_path_buf() } else { dir.join(file.file_stem().unwrap_or_default()) };
        let with_mods = mods_everywhere || seeds.contains(&file);
        // a `crate::` path resolves from any directory between the file and the crate root
        let bases: Vec<&Path> = std::iter::once(child_dir.as_path()).chain(dir.ancestors().take_while(|a| a.starts_with(crate_root) || *a == dir)).chain([crate_root]).collect();
        for name in info.uses.iter().chain(info.mods.iter().filter(|_| with_mods)) {
            for base in bases.iter().copied() {
                for cand in [base.join(format!("{name}.wj")), base.join(name).join("mod.wj")] {
                    if cand.is_file() && seen.insert(cand.clone()) {
                        queue.push(cand);
                    }
                }
            }
        }
    }
    seen.into_iter().collect()
}

/// The module names a file's `use` declarations and bodiless `mod name;` declarations name (first segment each,
/// `crate` / `self` / `super` prefixes skipped).
fn imported_names(text: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let (mut uses, mut mods) = (BTreeSet::new(), BTreeSet::new());
    for line in text.lines() {
        let l = line.trim_start();
        let l = l.strip_prefix("pub ").unwrap_or(l);
        let (rest, is_use) = match (l.strip_prefix("use "), l.strip_prefix("mod ")) {
            (Some(r), _) => (r, true),
            (_, Some(r)) => (r, false),
            _ => continue,
        };
        let mut segs = rest.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|s| !s.is_empty());
        let first = if is_use { segs.find(|s| !matches!(*s, "crate" | "self" | "super")) } else { segs.next() };
        let out = if is_use { &mut uses } else { &mut mods };
        if let Some(n) = first {
            out.insert(n.to_string());
        }
        if is_use {
            // `use crate::{a, b}` / `use {a, b}`: every identifier of a braced list may name a module
            if let Some(i) = rest.find('{') {
                for s in rest[i..].split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|s| !s.is_empty()) {
                    uses.insert(s.to_string());
                }
            }
        }
    }
    (uses, mods)
}
