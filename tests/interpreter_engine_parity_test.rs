#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! ADR-0020 kill criterion 2: `--engine=resolved` must be indistinguishable from the tree walker.
//! Every program of the interpreter corpus (every seed), the cross-backend conformance corpus,
//! `examples/verified` and the spec examples runs under both engines; stdout, stderr (locations
//! and backtraces included) and the exit status must be identical.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn wj_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            wj_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "wj") {
            out.push(p);
        }
    }
}

fn run(path: &Path, engine: &str, cwd: &Path) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let o = Command::new(WJ)
        .args(["run", "--interpret", "--no-lint", "--deterministic-time", "--engine", engine])
        .arg(path)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    (o.stdout, o.stderr, o.status.code())
}

#[test]
fn resolved_engine_matches_the_walker_on_every_corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tmp = tempfile::TempDir::new().unwrap();
    let mut programs = Vec::new();
    for d in ["tests/conformance_programs", "examples/verified", "tests/spec", "tests/engine_parity"] {
        wj_files(&root.join(d), &mut programs);
    }
    // the interpreter corpus is seeded: instantiate every seed
    let mut corpus = Vec::new();
    wj_files(&root.join("tests/interpreter_corpus"), &mut corpus);
    for p in corpus.iter().filter(|p| !p.to_string_lossy().contains("selftest")) {
        let src = fs::read_to_string(p).unwrap();
        let seeds = src.lines().find_map(|l| l.strip_prefix("// seeds:")).unwrap_or("1").to_string();
        for s in seeds.split_whitespace() {
            let f = tmp.path().join(format!("{}_{}.wj", p.file_stem().unwrap().to_string_lossy(), s));
            fs::write(&f, src.replace("__SEED__", s)).unwrap();
            programs.push(f);
        }
    }
    let mut diffs = Vec::new();
    let mut compared = 0;
    for p in &programs {
        let src = fs::read_to_string(p).unwrap_or_default();
        if !src.contains("fn main") || src.contains("stdin") {
            continue;
        }
        let a = run(p, "walker", tmp.path());
        let b = run(p, "resolved", tmp.path());
        compared += 1;
        if a != b {
            diffs.push(format!(
                "{}:\n--- walker (exit {:?})\n{}{}\n--- resolved (exit {:?})\n{}{}",
                p.display(),
                a.2,
                String::from_utf8_lossy(&a.0),
                String::from_utf8_lossy(&a.1),
                b.2,
                String::from_utf8_lossy(&b.0),
                String::from_utf8_lossy(&b.1)
            ));
        }
    }
    assert!(compared > 300, "too few programs compared: {compared}");
    assert!(diffs.is_empty(), "{} of {} programs differ between engines:\n{}", diffs.len(), compared, diffs.join("\n=====\n"));
}

/// The one documented difference (ADR-0020, step 1): in a program rustc rejects, a function declared to
/// return an integer that returns something else. The walker carries the wrong-typed value on; a typed
/// call site of the resolved engine reports the mismatch.
#[test]
fn resolved_engine_reports_an_ill_typed_integer_result() {
    let tmp = tempfile::TempDir::new().unwrap();
    let p = tmp.path().join("ill_typed.wj");
    fs::write(&p, "fn twice(x: int) -> int {\n    x + x\n}\nfn main() {\n    let s = \"ab\"\n    println(\"${twice(s)}\")\n}\n").unwrap();
    let (out, err, code) = run(&p, "resolved", tmp.path());
    assert_eq!(code, Some(1), "stdout: {}", String::from_utf8_lossy(&out));
    assert!(String::from_utf8_lossy(&err).contains("mismatched types: expected `i64`, found `String`"));
}
