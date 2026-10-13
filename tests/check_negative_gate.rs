//! Negative-corpus gate for `wj check` (differential against rustc, recorded oracle).
//!
//! Corpus: type-error mutants of the `// rust: ok` verified examples
//! (`scripts/gen-check-mutants.py` -> tests/check_negative/mutants), the diagnostics cases and the
//! error-system corpora. `scripts/check-negative-oracle.sh` records, per program, whether
//! `wj build --no-cargo` + `cargo check` accepts it (tests/check_negative/oracle.tsv). This test never
//! runs cargo: whenever the build+rustc rejects a program, `wj check` must report an error, unless the
//! program is listed in tests/check_negative/known_false_negatives.txt (a ratchet: entries that start
//! being caught must be removed, so the list only shrinks).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use windjammer::check::check_sources;
use windjammer::diagnostics::Origin;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct Entry {
    sha: String,
    transpile_ok: bool,
    rustc_ok: bool,
}

impl Entry {
    /// The real pipeline (front end + rustc) rejects the program.
    fn rejected(&self) -> bool {
        !self.transpile_ok || !self.rustc_ok
    }
}

fn oracle() -> BTreeMap<String, Entry> {
    let text = fs::read_to_string(root().join("tests/check_negative/oracle.tsv")).expect("oracle.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            assert_eq!(c.len(), 4, "bad oracle line: {l}");
            (
                c[0].to_string(),
                Entry { sha: c[1].to_string(), transpile_ok: c[2] == "ok", rustc_ok: c[3] == "ok" },
            )
        })
        .collect()
}

fn known_false_negatives() -> BTreeMap<String, String> {
    fs::read_to_string(root().join("tests/check_negative/known_false_negatives.txt"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#') && l.contains('|'))
        .map(|l| {
            let (p, r) = l.split_once('|').unwrap();
            assert!(!r.trim().is_empty(), "entry without a reason: {l}");
            (p.trim().to_string(), r.trim().to_string())
        })
        .collect()
}

fn corpus_files() -> Vec<String> {
    let mut v = Vec::new();
    for dir in ["tests/check_negative/mutants", "tests/diagnostics/cases", "tests/error_system", "tests/error_system_e2e"] {
        for e in fs::read_dir(root().join(dir)).unwrap().flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "wj") {
                v.push(format!("{dir}/{}", p.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    v.sort();
    v
}

/// FNV-1a 64 of the file bytes (same as scripts/check-negative-oracle.sh).
fn sha16(path: &Path) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in fs::read(path).unwrap() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// `wj check` (in-process) reports at least one error.
fn check_reports_error(rel: &str) -> bool {
    let text = fs::read_to_string(root().join(rel)).unwrap();
    check_sources(Path::new("."), &[(rel.to_string(), text)]).iter().any(|d| d.is_error())
}

/// Semantic (non-lint) errors only: what a rustc-accepted program must not get.
fn check_reports_semantic_error(rel: &str) -> bool {
    let text = fs::read_to_string(root().join(rel)).unwrap();
    check_sources(Path::new("."), &[(rel.to_string(), text)])
        .iter()
        .any(|d| d.is_error() && d.origin == Origin::FrontEnd && d.code != "WJ0023")
}

struct Outcome {
    rejected: usize,
    false_negatives: Vec<String>,
}

fn evaluate(o: &BTreeMap<String, Entry>, catches: &dyn Fn(&str) -> bool) -> Outcome {
    let mut out = Outcome { rejected: 0, false_negatives: Vec::new() };
    for (path, e) in o {
        if e.rejected() {
            out.rejected += 1;
            if !catches(path) {
                out.false_negatives.push(path.clone());
            }
        }
    }
    out
}

#[test]
fn oracle_covers_exactly_the_current_corpus() {
    let o = oracle();
    let files = corpus_files();
    let mut stale = Vec::new();
    for f in &files {
        match o.get(f) {
            None => stale.push(format!("{f}: not in oracle")),
            Some(e) if e.sha != sha16(&root().join(f)) => stale.push(format!("{f}: source changed")),
            _ => {}
        }
    }
    for k in o.keys() {
        if !files.contains(k) {
            stale.push(format!("{k}: in oracle but no longer in the corpus"));
        }
    }
    assert!(stale.is_empty(), "re-run scripts/check-negative-oracle.sh:\n{}", stale.join("\n"));
}

#[test]
fn check_catches_everything_rustc_rejects_except_ratcheted_entries() {
    let o = oracle();
    let known = known_false_negatives();
    let r = evaluate(&o, &check_reports_error);
    let unexpected: Vec<_> = r.false_negatives.iter().filter(|p| !known.contains_key(*p)).collect();
    let fixed: Vec<_> = known
        .keys()
        .filter(|k| o.get(*k).is_some_and(|e| e.rejected()) && !r.false_negatives.contains(k))
        .collect();
    let gone: Vec<_> = known.keys().filter(|k| !o.get(*k).is_some_and(|e| e.rejected())).collect();
    let fn_rate = r.false_negatives.len() as f64 / r.rejected.max(1) as f64;
    let before = o.values().filter(|e| e.rejected() && e.transpile_ok).count();
    eprintln!(
        "negative gate: {} rejected programs; wj check misses {} ({:.1}%); a transpile-only check misses {} ({:.1}%)",
        r.rejected,
        r.false_negatives.len(),
        100.0 * fn_rate,
        before,
        100.0 * before as f64 / r.rejected.max(1) as f64
    );
    assert!(unexpected.is_empty(), "new false negatives (rustc rejects, wj check is silent):\n{unexpected:#?}");
    assert!(fixed.is_empty(), "now caught; remove from known_false_negatives.txt:\n{fixed:#?}");
    assert!(gone.is_empty(), "known_false_negatives.txt lists programs rustc accepts or that are gone:\n{gone:#?}");
}

#[test]
fn no_semantic_errors_on_programs_rustc_accepts() {
    let o = oracle();
    let fps: Vec<_> = o
        .iter()
        .filter(|(_, e)| !e.rejected())
        .filter(|(p, _)| check_reports_semantic_error(p))
        .map(|(p, _)| p.clone())
        .collect();
    assert!(fps.is_empty(), "wj check rejects programs the build + rustc accept:\n{fps:#?}");
}

/// The gate must be able to fail: a checker that never reports anything is caught on every
/// rejected program, and every mutant really differs from its source example.
#[test]
fn mutation_self_test() {
    let o = oracle();
    let silent = evaluate(&o, &|_| false);
    assert!(silent.rejected >= 50, "corpus too small to mean anything: {}", silent.rejected);
    assert_eq!(silent.false_negatives.len(), silent.rejected, "a silent checker must miss every rejected program");
    let loud = evaluate(&o, &|_| true);
    assert!(loud.false_negatives.is_empty());
    for f in corpus_files().iter().filter(|f| f.contains("/mutants/")) {
        let text = fs::read_to_string(root().join(f)).unwrap();
        let src = text.lines().next().and_then(|l| l.split(" of ").nth(1)).expect("mutant header");
        let original: String = fs::read_to_string(root().join(src))
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with("// "))
            .collect::<Vec<_>>()
            .join("\n");
        let body: String = text.lines().skip(1).collect::<Vec<_>>().join("\n");
        assert_ne!(body.trim(), original.trim(), "{f} is identical to {src}");
    }
}
