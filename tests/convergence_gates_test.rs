//! Convergence gates (docs/spec/convergence-plan.md, step C0). Every ownership / clone / width
//! change is judged by these three gates before it merges:
//!
//!   1. **Golden emitted Rust.** Every corpus program is compiled with `wj build --no-cargo`; the
//!      output must equal `tests/golden/rust/<path>.rs`. Refresh with `UPDATE_GOLDEN=1` and review the
//!      diff (each change must be explained by a divergence ID in the commit message).
//!   2. **rustc accepts the goldens** (independent oracle): `rustc --emit=metadata` on every golden.
//!      Files rustc rejects are listed in `tests/golden/rustc_rejects.txt`; the list may only shrink.
//!   3. **Explain ratchet.** `wj explain-ownership` findings (by ID) summed over the corpus may not
//!      rise above `tests/golden/explain_counts.txt`.
//!
//! Standalone: `cargo test --release --test convergence_gates -- --test-threads 1` (gate 3 reads the
//! goldens gate 1 writes). First baseline: `UPDATE_GOLDEN=1 WJ_GATE_BASELINE=1`. `WJ_GATE_JOBS` sets parallelism
//! (default 2). `WJ_GATE_FILTER=<substring>` restricts the corpus (gates 2/3 then only check, never
//! rewrite, and skip the "list may only shrink" direction for files outside the filter).
//!
//! consumes: examples/verified/*.wj
//! consumes: tests/conformance/*.wj
//! consumes: tests/conformance/*/*.wj
//! consumes: tests/inference_corpus/*.wj
//! consumes: tests/auto_clone/*.wj
//! consumes: tests/*.wj
//! consumes: tests/golden/rust/**

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const CORPUS_DIRS: &[&str] = &[
    "examples/verified",
    "tests/conformance",
    "tests/conformance/basics",
    "tests/conformance/control_flow",
    "tests/conformance/error_handling",
    "tests/conformance/stdlib",
    "tests/conformance/types",
    "tests/conformance/values",
    "tests/inference_corpus",
    "tests/auto_clone",
    "tests",
];

const GOLDEN_DIR: &str = "tests/golden/rust";
const RUSTC_REJECTS: &str = "tests/golden/rustc_rejects.txt";
const EXPLAIN_COUNTS: &str = "tests/golden/explain_counts.txt";
/// Golden content for a program the compiler rejects (kept so a newly rejected program is a diff).
const BUILD_FAILED: &str = "// wj build failed\n";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn updating() -> bool {
    std::env::var("UPDATE_GOLDEN").is_ok_and(|v| v == "1")
}

fn filter() -> Option<String> {
    std::env::var("WJ_GATE_FILTER").ok().filter(|s| !s.is_empty())
}

fn corpus() -> Vec<String> {
    let mut out = Vec::new();
    for dir in CORPUS_DIRS {
        let Ok(rd) = std::fs::read_dir(root().join(dir)) else { continue };
        let mut files: Vec<String> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "wj"))
            .map(|p| format!("{dir}/{}", p.file_name().unwrap().to_string_lossy()))
            .collect();
        files.sort();
        out.extend(files);
    }
    if let Some(f) = filter() {
        out.retain(|p| p.contains(&f));
    }
    out
}

fn jobs() -> usize {
    std::env::var("WJ_GATE_JOBS").ok().and_then(|v| v.parse().ok()).unwrap_or(2).max(1)
}

/// Run `work` over `items` on `jobs()` threads, preserving order.
fn parallel<T: Sync, R: Send>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<R>>> = Mutex::new((0..items.len()).map(|_| None).collect());
    std::thread::scope(|s| {
        for _ in 0..jobs() {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= items.len() {
                    break;
                }
                let r = work(&items[i]);
                results.lock().unwrap()[i] = Some(r);
            });
        }
    });
    results.into_inner().unwrap().into_iter().map(|r| r.unwrap()).collect()
}

/// Emitted Rust for one corpus program: every generated `.rs` file, sorted, concatenated.
fn emit(rel: &str) -> String {
    let src = root().join(rel);
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join(src.file_name().unwrap());
    std::fs::copy(&src, &file).unwrap();
    let out = tmp.path().join("out");
    let st = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&file)
        .args(["--target", "rust", "--no-cargo", "--no-lint", "--quiet", "-o"])
        .arg(&out)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    if !st.status.success() {
        return BUILD_FAILED.to_string();
    }
    let mut rs: Vec<PathBuf> = walk(&out).into_iter().filter(|p| p.extension().is_some_and(|e| e == "rs")).collect();
    rs.sort();
    if rs.is_empty() {
        return BUILD_FAILED.to_string();
    }
    let mut text = String::new();
    for p in rs {
        let rel = p.strip_prefix(&out).unwrap().display().to_string();
        if !text.is_empty() {
            text.push_str(&format!("\n// ==== {rel} ====\n"));
        }
        text.push_str(&std::fs::read_to_string(&p).unwrap_or_default());
    }
    text
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

fn golden_path(rel: &str) -> PathBuf {
    root().join(GOLDEN_DIR).join(format!("{rel}.rs"))
}

/// `Ok` when rustc type/borrow-checks the golden (metadata only, no codegen).
fn rustc_accepts(rust: &str) -> Result<(), String> {
    if rust == BUILD_FAILED {
        return Err("wj build failed".into());
    }
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("main.rs");
    std::fs::write(&f, rust).unwrap();
    // Library-shaped programs (no `main`) are checked as a lib crate.
    let crate_type = if rust.contains("fn main(") { "bin" } else { "lib" };
    let o = Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", crate_type, "--emit=metadata", "-A", "warnings", "--crate-name", "golden", "-o"])
        .arg(tmp.path().join("out.rmeta"))
        .arg(&f)
        .output()
        .unwrap();
    if o.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&o.stderr);
        Err(err.lines().find(|l| l.starts_with("error")).unwrap_or("error").to_string())
    }
}

fn read_list(path: &str) -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(root().join(path)).unwrap_or_default();
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| match l.split_once(" | ") {
            Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
            None => (l.to_string(), String::new()),
        })
        .collect()
}

fn write_rejects(map: &BTreeMap<String, String>) {
    let mut s = String::from(
        "# Golden programs rustc rejects (tests/convergence_gates_test.rs, gate 2). May only shrink.\n\
         # <corpus path> | <first rustc error>\n",
    );
    for (k, v) in map {
        s.push_str(&format!("{k} | {v}\n"));
    }
    std::fs::write(root().join(RUSTC_REJECTS), s).unwrap();
}

/// Gate 1: emitted Rust equals the goldens. Gate 2 runs on the same outputs.
#[test]
fn gate1_golden_emitted_rust_and_rustc_accepts() {
    let files = corpus();
    assert!(files.len() > 100 || filter().is_some(), "corpus unexpectedly small: {}", files.len());
    let emitted = parallel(&files, |rel| emit(rel));

    // Gate 1.
    let mut diffs = Vec::new();
    for (rel, rust) in files.iter().zip(&emitted) {
        let gp = golden_path(rel);
        let old = std::fs::read_to_string(&gp).ok();
        if old.as_deref() != Some(rust.as_str()) {
            if updating() {
                std::fs::create_dir_all(gp.parent().unwrap()).unwrap();
                std::fs::write(&gp, rust).unwrap();
            }
            diffs.push(rel.clone());
        }
    }
    if updating() && filter().is_none() {
        // Drop goldens whose source left the corpus.
        let live: BTreeSet<PathBuf> = files.iter().map(|r| golden_path(r)).collect();
        for p in walk(&root().join(GOLDEN_DIR)) {
            if !live.contains(&p) {
                let _ = std::fs::remove_file(p);
            }
        }
    }

    // Gate 2.
    let verdicts = parallel(&files.iter().zip(&emitted).collect::<Vec<_>>(), |(_, rust)| rustc_accepts(rust));
    let known = read_list(RUSTC_REJECTS);
    let mut newly_rejected = Vec::new();
    let mut now_accepted = Vec::new();
    let mut current = BTreeMap::new();
    for (rel, v) in files.iter().zip(&verdicts) {
        match (v, known.contains_key(rel)) {
            (Err(e), false) => newly_rejected.push(format!("  {rel}: {e}")),
            (Ok(()), true) => now_accepted.push(format!("  {rel}")),
            _ => {}
        }
        if let Err(e) = v {
            current.insert(rel.clone(), e.chars().take(160).collect::<String>());
        }
    }
    // UPDATE_GOLDEN=1 may only shrink the list; adding entries needs WJ_GATE_BASELINE=1 (first
    // baseline, or a reviewed decision recorded in the commit message).
    let baseline = std::env::var("WJ_GATE_BASELINE").is_ok_and(|v| v == "1");
    if updating() && filter().is_none() && (newly_rejected.is_empty() || baseline) {
        write_rejects(&current);
        if baseline {
            newly_rejected.clear();
        }
    }

    let mut msg = String::new();
    if !diffs.is_empty() && !updating() {
        msg.push_str(&format!(
            "{} golden(s) differ (review, then UPDATE_GOLDEN=1):\n  {}\n",
            diffs.len(),
            diffs.join("\n  ")
        ));
    }
    if !newly_rejected.is_empty() {
        msg.push_str(&format!(
            "rustc now rejects {} golden(s) (regression; fix the compiler, do not list them):\n{}\n",
            newly_rejected.len(),
            newly_rejected.join("\n")
        ));
    }
    if !now_accepted.is_empty() && !updating() {
        msg.push_str(&format!(
            "rustc now accepts {} listed golden(s): rerun with UPDATE_GOLDEN=1 to shrink {RUSTC_REJECTS}:\n{}\n",
            now_accepted.len(),
            now_accepted.join("\n")
        ));
    }
    assert!(msg.is_empty(), "{msg}");
}

/// C0 kill criterion: emission must be deterministic. Opt-in (`WJ_GATE_DETERMINISM=N` builds every
/// program N extra times, default off) because it multiplies the gate's cost.
#[test]
fn gate0_emission_is_deterministic() {
    let runs: usize = std::env::var("WJ_GATE_DETERMINISM").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    if runs == 0 {
        return;
    }
    let files = corpus();
    let first = parallel(&files, |rel| emit(rel));
    let mut unstable = BTreeSet::new();
    for _ in 0..runs {
        let again = parallel(&files, |rel| emit(rel));
        for ((rel, a), b) in files.iter().zip(&first).zip(&again) {
            if a != b {
                unstable.insert(rel.clone());
            }
        }
    }
    assert!(unstable.is_empty(), "nondeterministic emission:\n  {}", unstable.into_iter().collect::<Vec<_>>().join("\n  "));
}

/// Finding counts by ID over the corpus, using the goldens as the emitted Rust.
fn explain_counts(files: &[String]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for rel in files {
        let src = std::fs::read_to_string(root().join(rel)).unwrap();
        let rust = std::fs::read_to_string(golden_path(rel)).ok().filter(|r| r != BUILD_FAILED);
        let opts = windjammer::ir::explain::ExplainOptions { only_fn: None, emitted_rust: rust };
        let outcome = std::panic::catch_unwind(|| windjammer::ir::explain::explain_source(&src, rel, &opts));
        match outcome {
            Ok(Ok(report)) => {
                for f in report.findings() {
                    *counts.entry(f.id.clone()).or_insert(0) += 1;
                }
                // C2: every constraint that reaches a report must carry a rule.
                let untagged = report
                    .functions
                    .iter()
                    .flat_map(|f| {
                        f.params.iter().flat_map(|p| p.reasons.iter())
                            .chain(f.locals.iter().flat_map(|l| l.reasons.iter()))
                            .chain(f.clones.iter().flat_map(|c| c.reasons.iter()))
                    })
                    .filter(|r| r.rule == "UNTAGGED")
                    .count();
                if untagged > 0 {
                    *counts.entry("UNTAGGED-REASON".into()).or_insert(0) += untagged;
                }
            }
            Ok(Err(_)) => *counts.entry("EXPLAIN-ERROR".into()).or_insert(0) += 1,
            Err(_) => *counts.entry("EXPLAIN-PANIC".into()).or_insert(0) += 1,
        }
    }
    counts
}

/// Gate 3: no finding count may rise.
#[test]
fn gate3_explain_finding_counts_do_not_rise() {
    let files = corpus();
    let counts = explain_counts(&files);
    if filter().is_some() {
        for (k, v) in &counts {
            eprintln!("{k} {v}");
        }
        return;
    }
    let baseline: BTreeMap<String, usize> = read_list(EXPLAIN_COUNTS)
        .into_iter()
        .map(|(k, v)| (k, v.parse().unwrap_or(0)))
        .collect();
    if updating() {
        let mut s = String::from(
            "# `wj explain-ownership` finding counts over the convergence corpus (gate 3). May only fall.\n\
             # <finding id> | <count>\n",
        );
        for (k, v) in &counts {
            s.push_str(&format!("{k} | {v}\n"));
        }
        std::fs::write(root().join(EXPLAIN_COUNTS), s).unwrap();
    }
    let mut rose = Vec::new();
    let mut fell = Vec::new();
    let keys: BTreeSet<&String> = counts.keys().chain(baseline.keys()).collect();
    for k in keys {
        let now = counts.get(k).copied().unwrap_or(0);
        let was = baseline.get(k).copied().unwrap_or(0);
        if now > was {
            rose.push(format!("  {k}: {was} -> {now}"));
        } else if now < was {
            fell.push(format!("  {k}: {was} -> {now}"));
        }
    }
    for f in &fell {
        eprintln!("explain count fell (rerun with UPDATE_GOLDEN=1 to lock in):{f}");
    }
    assert!(updating() || rose.is_empty(), "explain finding counts rose:\n{}", rose.join("\n"));
}
