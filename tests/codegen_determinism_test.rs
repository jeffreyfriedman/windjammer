//! Codegen determinism gate: the same source must produce byte-identical output on every build.
//!
//! Found by inspection before this gate existed: `v.contains(15)` was emitted with `&` in 3 of 6
//! identical builds (HashMap iteration order feeding codegen). Two layers:
//!
//! 1. Dynamic: every verified example and conformance program is built N times with
//!    `wj build --no-cargo` (Rust; Go/JS/TS under WJ_DETERMINISM_FULL=1) in fresh processes (fresh
//!    std `RandomState` seeds) with different rayon thread counts; every output file (`.rs`, `.go`,
//!    `.mjs`, `.ts`, `Cargo.toml`, `.wj.meta`, ...) must hash identically. Known nondeterministic
//!    programs are listed in tests/determinism_known.txt (ratchet: an entry that becomes stable fails).
//! 2. Static: iteration over `HashMap`/`HashSet` in src/codegen, src/ir and src/analyzer (where it
//!    can feed emitted text) is counted per file in tests/determinism_hash_iteration.toml; counts may
//!    only go down (`DETERMINISM_BLESS=1` tightens).

#[path = "common/hash_iteration_lint.rs"]
mod hash_iteration_lint;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn corpus() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for d in ["examples/verified", "tests/conformance_programs"] {
        for e in std::fs::read_dir(root().join(d)).unwrap().flatten() {
            if e.path().extension().is_some_and(|x| x == "wj") {
                v.push(e.path());
            }
        }
    }
    v.sort();
    v
}

/// A generated crate's version carries `+wj<16 hex digits>`, a hash of its own canonical output directory (so two
/// same-named projects never share a cargo build, see `cargo_cache::crate_version`). It differs between the fresh
/// temp dirs of two builds by design, so it is masked like the temp path itself.
fn mask_crate_identity(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("+wj") {
        let (head, tail) = rest.split_at(i + 3);
        out.push_str(head);
        let hex = tail.bytes().take_while(|b| b.is_ascii_hexdigit()).count();
        if hex == 16 {
            out.push_str("<ID>");
            rest = &tail[16..];
        } else {
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

fn hash_tree(dir: &Path, base: &Path, scratch: &str, out: &mut BTreeMap<String, u64>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            hash_tree(&p, base, scratch, out);
        } else {
            let text = std::fs::read(&p).unwrap_or_default();
            // Normalize the per-run temp dir so only real content differences count.
            let text = String::from_utf8_lossy(&text).replace(base.to_str().unwrap(), "<OUT>").replace(scratch, "<TMP>");
            let text = mask_crate_identity(&text);
            let h = text.bytes().fold(1469598103934665603u64, |h, b| (h ^ b as u64).wrapping_mul(1099511628211));
            out.insert(p.strip_prefix(base).unwrap().to_string_lossy().into_owned(), h);
        }
    }
}

/// One build in a fresh process; `None` when the build fails (not a determinism question).
fn build_once(src: &Path, target: &str, threads: usize) -> Option<BTreeMap<String, u64>> {
    let tmp = tempfile::TempDir::new().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let out = base.join("out");
    // Hermetic input: `wj build` loads peer metadata from `<project root>/.wj-cache/` (the nearest dir with a
    // Cargo.toml/wj.toml above the source), a cache that every earlier build of any file in the checkout
    // writes. Building in place made a program's output depend on which builds ran before it (cold vs warm
    // cache; concurrent tests). A private copy under its own project root has no shared state.
    let proj = base.join("proj");
    std::fs::create_dir_all(proj.join("src")).unwrap();
    std::fs::write(proj.join("wj.toml"), "[package]\nname = \"determinism_probe\"\nversion = \"0.0.0\"\n").unwrap();
    let local = proj.join("src").join(src.file_name().unwrap());
    std::fs::copy(src, &local).unwrap();
    let st = Command::new(WJ)
        .args(["build", "--target", target, "--no-cargo", "--no-lint", "-o"])
        .arg(&out)
        .arg(&local)
        .env("RAYON_NUM_THREADS", threads.to_string())
        .current_dir(&base)
        .output()
        .unwrap();
    if !st.status.success() {
        return None;
    }
    let mut h = BTreeMap::new();
    hash_tree(&out, &out, base.to_str().unwrap(), &mut h);
    Some(h)
}

/// Files whose content differs between runs, or `None` if every successful run agrees.
pub fn divergence(runs: &[Option<BTreeMap<String, u64>>]) -> Option<String> {
    let ok: Vec<_> = runs.iter().flatten().collect();
    if ok.len() != runs.len() && !ok.is_empty() {
        return Some(format!("build succeeded in {} of {} runs", ok.len(), runs.len()));
    }
    let first = ok.first()?;
    for r in &ok[1..] {
        if r != first {
            let diff: Vec<_> = first.keys().chain(r.keys()).filter(|k| first.get(*k) != r.get(*k)).cloned().collect();
            return Some(format!("differs in {diff:?}"));
        }
    }
    None
}

fn known() -> Vec<String> {
    std::fs::read_to_string(root().join("tests/determinism_known.txt"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| l.split('|').next().unwrap().trim().to_string())
        .collect()
}

#[test]
fn every_build_is_byte_identical_across_runs_and_thread_counts() {
    let full = std::env::var("WJ_DETERMINISM_FULL").is_ok();
    let targets: &[&str] = if full { &["rust", "go", "javascript", "typescript"] } else { &["rust"] };
    let runs = if full { 6 } else { 3 };
    let files = corpus();
    assert!(files.len() > 200, "corpus too small: {}", files.len());
    let mut unstable = BTreeMap::new();
    std::thread::scope(|s| {
        let handles: Vec<_> = files
            .chunks(files.len().div_ceil(4))
            .map(|chunk| {
                s.spawn(move || {
                    let mut found = Vec::new();
                    for f in chunk {
                        for t in targets {
                            let r: Vec<_> = (0..runs).map(|i| build_once(f, t, [1, 4, 2, 8, 3, 16][i % 6])).collect();
                            if let Some(d) = divergence(&r) {
                                let rel = f.strip_prefix(root()).unwrap().to_string_lossy().into_owned();
                                found.push((format!("{rel} [{t}]"), d));
                            }
                        }
                    }
                    found
                })
            })
            .collect();
        for h in handles {
            unstable.extend(h.join().unwrap());
        }
    });
    let known = known();
    let new: Vec<_> = unstable.iter().filter(|(k, _)| !known.contains(k)).map(|(k, d)| format!("{k}: {d}")).collect();
    // Stability is probabilistic, so a known entry is only reported stale in the thorough mode.
    let stale: Vec<_> = known.iter().filter(|k| full && !unstable.contains_key(*k)).collect();
    assert!(new.is_empty(), "NONDETERMINISTIC codegen (same input, different output):\n{}", new.join("\n"));
    assert!(stale.is_empty(), "now deterministic, remove from tests/determinism_known.txt: {stale:?}");
}

/// One `--library --module-file` build of a multi-module library (the multipass path, which also writes
/// `metadata.json`) in a fresh process; `None` when the build fails.
fn build_library_once(threads: usize) -> Option<BTreeMap<String, u64>> {
    let tmp = tempfile::TempDir::new().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let proj = base.join("proj");
    let mut root = String::new();
    for i in 0..8 {
        root.push_str(&format!("pub mod m{i}\n"));
        let prev = if i > 0 { format!("use crate::m{}::Unit{}\n", i - 1, i - 1) } else { String::new() };
        let follow = if i > 0 { format!("    pub fn follow(self, other: Unit{}) -> f32 {{\n        other.speed - self.speed\n    }}\n", i - 1) } else { String::new() };
        let module = format!(
            "{prev}\npub enum Kind{i} {{\n    Idle,\n    Wait(f32),\n    Tag(string),\n}}\n\npub struct Unit{i} {{\n    pub id: int,\n    pub speed: f32,\n    pub name: string,\n    pub tags: Vec<string>,\n    pub kind: Kind{i},\n}}\n\nimpl Unit{i} {{\n    pub fn new(id: int) -> Unit{i} {{\n        Unit{i} {{ id: id, speed: 1.0, name: \"u\", tags: Vec::new(), kind: Kind{i}::Idle }}\n    }}\n\n    pub fn rename(self, name: string) {{\n        self.name = name\n    }}\n{follow}}}\n"
        );
        std::fs::create_dir_all(proj.join("src")).unwrap();
        std::fs::write(proj.join("src").join(format!("m{i}.wj")), module).unwrap();
    }
    std::fs::write(proj.join("src/mod.wj"), root).unwrap();
    std::fs::write(proj.join("wj.toml"), "[package]\nname = \"determinism_lib\"\nversion = \"0.0.0\"\n").unwrap();
    let st = Command::new(WJ)
        .args(["build", "src/mod.wj", "--output", "gen", "--library", "--no-cargo", "--module-file", "--no-lint"])
        .env("RAYON_NUM_THREADS", threads.to_string())
        .current_dir(&proj)
        .output()
        .unwrap();
    if !st.status.success() {
        return None;
    }
    let out = proj.join("gen");
    let mut h = BTreeMap::new();
    hash_tree(&out, &out, base.to_str().unwrap(), &mut h);
    Some(h)
}

/// Library mode (multipass) is byte-identical across runs and thread counts, metadata.json included (its maps
/// were written in hash order, so every build of the same library differed).
#[test]
fn library_build_is_byte_identical_across_runs_and_thread_counts() {
    let runs: Vec<_> = [1, 4, 1, 8].iter().map(|t| build_library_once(*t)).collect();
    assert!(runs[0].as_ref().is_some_and(|h| h.contains_key("metadata.json")), "library build failed or wrote no metadata.json");
    if let Some(d) = divergence(&runs) {
        panic!("library build output is nondeterministic: {d}");
    }
}

#[test]
fn hash_iteration_in_codegen_paths_only_shrinks() {
    hash_iteration_lint::check_ratchet(&root());
}

// --- self-tests -------------------------------------------------------------------------------

#[test]
fn selftest_divergence_detects_differing_output() {
    let a: BTreeMap<String, u64> = [("m.rs".to_string(), 1)].into();
    let b: BTreeMap<String, u64> = [("m.rs".to_string(), 2)].into();
    assert!(divergence(&[Some(a.clone()), Some(a.clone())]).is_none());
    assert!(divergence(&[Some(a.clone()), Some(b)]).unwrap().contains("m.rs"));
    assert!(divergence(&[Some(a), None]).is_some(), "flaky success is nondeterminism too");
    assert!(divergence(&[None, None]).is_none());
}

#[test]
fn selftest_lint_flags_hash_iteration_and_ignores_ordered_maps() {
    let src = r#"
use std::collections::{HashMap, BTreeMap};
struct G { by_name: HashMap<String, u32>, ordered: BTreeMap<String, u32> }
fn emit(g: &G, out: &mut String) {
    for (k, v) in &g.by_name { out.push_str(k); }
    let seen: HashSet<String> = HashSet::new();
    let names: Vec<_> = seen.iter().collect();
    for (k, _) in &g.ordered { out.push_str(k); }
    let x = g.by_name.get("a");
}
"#;
    let hits = hash_iteration_lint::scan(src);
    assert_eq!(hits.len(), 2, "{hits:?}");
}
