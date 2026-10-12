//! Differential gate: `wj check` versus the real pipeline (`wj build --no-cargo` + `rustc`).
//!
//! Promise: if the build pipeline rejects a program, `wj check` reports an error (no "check passed,
//! build fails"), and a program the pipeline accepts is not rejected by `wj check`. Historical
//! miss: `wj check` reported success on `let x: int = "s"`.
//!
//! Programs: a seeded generator instantiates a valid base program and applies one of a set of
//! mutation operators, each a known-invalid edit (labelled oracle: the program MUST be rejected),
//! plus unmutated controls and the interpreter corpus. Three classes of finding:
//! - `check-miss`: pipeline rejects, `wj check` passes;
//! - `check-false-error`: pipeline accepts, `wj check` fails;
//! - `pipeline-accepts-invalid`: an operator's invalid program builds AND compiles with rustc
//!   (the compiler silently accepted a type error).
//!
//! Ratchet: tests/check_vs_build_known.toml, counts per `class.operator`; must match exactly
//! (`CHECK_DIFF_BLESS=1` rewrites when nothing grew). `wj check` itself is owned by wt/wj-check.

#[path = "common/check_diff_gen.rs"]
mod check_diff_gen;

use check_diff_gen::{instantiate, OPERATORS};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");
const SEEDS: [u64; 3] = [7, 1009, 424242];
const KNOWN: &str = "tests/check_vs_build_known.toml";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `true` when the pipeline (wj build --no-cargo, then rustc type/borrow check) accepts.
fn pipeline_accepts(dir: &Path, src: &Path) -> bool {
    let out = dir.join("out");
    let b = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&out).arg(src).current_dir(dir).output().unwrap();
    if !b.status.success() {
        return false;
    }
    let rs = out.join(format!("{}.rs", src.file_stem().unwrap().to_string_lossy()));
    if !rs.exists() {
        return false;
    }
    let r = Command::new("rustc")
        .args(["--edition", "2021", "-A", "warnings", "--emit=metadata", "--crate-type=bin", "-o"])
        .arg(dir.join("m.rmeta"))
        .arg(&rs)
        .output()
        .unwrap();
    r.status.success()
}

fn check_accepts(dir: &Path, src: &Path) -> bool {
    let c = Command::new(WJ).arg("check").arg(src).current_dir(dir).env("NO_COLOR", "1").output().unwrap();
    c.status.success()
}

/// `class.operator` of the finding for one program, if any. `must_reject`: labelled invalid.
pub fn classify(must_reject: bool, pipeline: bool, check: bool, op: &str) -> Option<String> {
    match (pipeline, check) {
        (false, true) => Some(format!("check-miss.{op}")),
        (true, false) => Some(format!("check-false-error.{op}")),
        (true, true) if must_reject => Some(format!("pipeline-accepts-invalid.{op}")),
        _ => None,
    }
}

fn programs() -> Vec<(String, bool, String)> {
    let mut v = Vec::new();
    for seed in SEEDS {
        v.push(("control".to_string(), false, instantiate(seed, None)));
        for op in OPERATORS {
            v.push((op.name.to_string(), true, instantiate(seed, Some(op))));
        }
    }
    // Valid corpus programs (first seed of each interpreter-corpus template) as extra controls.
    let mut corpus: Vec<_> = std::fs::read_dir(root().join("tests/interpreter_corpus")).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wj")).collect();
    corpus.sort();
    for p in corpus {
        let t = std::fs::read_to_string(&p).unwrap();
        if t.contains("// interp-only") || t.contains("// exit:") {
            continue;
        }
        v.push(("corpus".to_string(), false, t.replace("__SEED__", "1")));
    }
    v
}

#[test]
fn check_agrees_with_the_build_pipeline() {
    if Command::new("rustc").arg("--version").output().is_err() {
        eprintln!("rustc not available: check-vs-build gate skipped");
        return;
    }
    let progs = programs();
    let found = std::sync::Mutex::new(BTreeMap::<String, Vec<String>>::new());
    std::thread::scope(|s| {
        for chunk in progs.chunks(progs.len().div_ceil(4)) {
            let found = &found;
            s.spawn(move || {
                for (i, (op, must_reject, src)) in chunk.iter().enumerate() {
                    let tmp = tempfile::TempDir::new().unwrap();
                    let f = tmp.path().join("prog.wj");
                    std::fs::write(&f, src).unwrap();
                    let (p, c) = (pipeline_accepts(tmp.path(), &f), check_accepts(tmp.path(), &f));
                    if let Some(k) = classify(*must_reject, p, c, op) {
                        let first = src.lines().find(|l| l.contains("// MUTATED")).unwrap_or("").trim().to_string();
                        found.lock().unwrap().entry(k).or_default().push(format!("#{i} {first}"));
                    }
                }
            });
        }
    });
    let found = found.into_inner().unwrap();
    let got: BTreeMap<String, usize> = found.iter().map(|(k, v)| (k.clone(), v.len())).collect();
    let text = std::fs::read_to_string(root().join(KNOWN)).unwrap_or_default();
    let known: BTreeMap<String, usize> = toml::from_str::<BTreeMap<String, BTreeMap<String, usize>>>(&text).ok().and_then(|mut t| t.remove("known")).unwrap_or_default();
    let grew: Vec<String> = got.iter().filter(|(k, n)| **n > known.get(*k).copied().unwrap_or(0)).map(|(k, n)| format!("{k}: {n} (known {}) e.g. {:?}", known.get(k).copied().unwrap_or(0), &found[k][..found[k].len().min(2)])).collect();
    let shrank: Vec<String> = known.iter().filter(|(k, n)| got.get(*k).copied().unwrap_or(0) < **n).map(|(k, n)| format!("{k}: {} (known {n})", got.get(k).copied().unwrap_or(0))).collect();
    eprintln!("check-vs-build over {} programs: {got:?}", progs.len());
    if grew.is_empty() && std::env::var("CHECK_DIFF_BLESS").is_ok() {
        let mut s = String::from("# check-vs-build differential findings (tests/check_build_differential_test.rs). May only shrink.\n# class.operator = count over the generated seeds; CHECK_DIFF_BLESS=1 rewrites after an improvement.\n\n[known]\n");
        for (k, n) in &got {
            s.push_str(&format!("{k:?} = {n}\n"));
        }
        std::fs::write(root().join(KNOWN), s).unwrap();
        return;
    }
    assert!(grew.is_empty(), "NEW check/build disagreements:\n{}", grew.join("\n"));
    assert!(shrank.is_empty(), "fixed (tighten with CHECK_DIFF_BLESS=1): {}", shrank.join(", "));
}

#[test]
fn selftest_classification_and_generator_are_sound() {
    assert_eq!(classify(true, false, true, "x").as_deref(), Some("check-miss.x"));
    assert_eq!(classify(false, true, false, "x").as_deref(), Some("check-false-error.x"));
    assert_eq!(classify(true, true, true, "x").as_deref(), Some("pipeline-accepts-invalid.x"));
    assert_eq!(classify(true, false, false, "x"), None);
    assert_eq!(classify(false, true, true, "control"), None);
    // Seeded, deterministic, and every operator actually changes the program.
    assert_eq!(instantiate(5, None), instantiate(5, None));
    assert_ne!(instantiate(5, None), instantiate(6, None));
    for op in OPERATORS {
        let m = instantiate(5, Some(op));
        assert_ne!(m, instantiate(5, None), "operator {} is a no-op", op.name);
        assert!(m.contains("// MUTATED"), "operator {} must mark its edit", op.name);
    }
}
