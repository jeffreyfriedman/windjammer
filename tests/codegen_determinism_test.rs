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

fn hash_tree(dir: &Path, base: &Path, out: &mut BTreeMap<String, u64>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            hash_tree(&p, base, out);
        } else {
            let text = std::fs::read(&p).unwrap_or_default();
            // Normalize the per-run temp dir so only real content differences count.
            let text = String::from_utf8_lossy(&text).replace(base.to_str().unwrap(), "<OUT>");
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
    let st = Command::new(WJ)
        .args(["build", "--target", target, "--no-cargo", "--no-lint", "-o"])
        .arg(&out)
        .arg(src)
        .env("RAYON_NUM_THREADS", threads.to_string())
        .current_dir(&base)
        .output()
        .unwrap();
    if !st.status.success() {
        return None;
    }
    let mut h = BTreeMap::new();
    hash_tree(&out, &out, &mut h);
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
