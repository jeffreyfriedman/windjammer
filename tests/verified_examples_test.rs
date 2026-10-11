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

//! The verified example corpus (`examples/verified/*.wj`): every program's `// expect:` lines (an oracle
//! derived by hand or from a Python model, never copied from the interpreter) must equal the output of
//! `wj run --interpret`; `// rust: ok` programs must also compile with `wj build --no-cargo` + `rustc`
//! and print the same lines; every program is lint-clean; the manifest and docs index are not stale.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;
use windjammer::agent_index::verified::{load_examples, ExampleHeader, RustStatus};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/verified")
}

fn lines(o: &Output) -> Vec<String> {
    let s = String::from_utf8_lossy(&o.stdout).to_string();
    s.lines().map(|l| l.to_string()).collect()
}

/// Compare observed output with the header; returns a description of the first difference.
fn compare(h: &ExampleHeader, got: &[String], exit: i32) -> Option<String> {
    if exit != h.exit {
        return Some(format!("{}: exit {} (expected {})", h.name, exit, h.exit));
    }
    if got != h.expect.as_slice() {
        return Some(format!("{}: expected {:?}, got {:?}", h.name, h.expect, got));
    }
    None
}

fn parallel<T: Send + 'static>(items: Vec<T>, f: impl Fn(T) -> Option<String> + Send + Sync + 'static) -> Vec<String> {
    use std::sync::{Arc, Mutex};
    let f = Arc::new(f);
    let queue = Arc::new(Mutex::new(items));
    let failures = Arc::new(Mutex::new(Vec::new()));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let (q, fl, f) = (queue.clone(), failures.clone(), f.clone());
            std::thread::spawn(move || loop {
                let item = q.lock().unwrap().pop();
                match item {
                    Some(i) => {
                        if let Some(msg) = f(i) {
                            fl.lock().unwrap().push(msg);
                        }
                    }
                    None => break,
                }
            })
        })
        .collect();
    for w in workers {
        w.join().unwrap();
    }
    let mut v = failures.lock().unwrap().clone();
    v.sort();
    v
}

fn run_example(h: &ExampleHeader, path: &Path) -> Option<String> {
    if h.is_test() {
        let o = Command::new(WJ).args(["test", "--interpret"]).arg(path).output().unwrap();
        let text = String::from_utf8_lossy(&o.stdout).to_string();
        let want_pass = h.tests_passed.unwrap_or(0);
        let want_fail = h.tests_failed.unwrap_or(0);
        let summary = format!("{} passed; {} failed", want_pass, want_fail);
        if !text.contains(&summary) {
            return Some(format!("{}: expected `{}` in `{}`", h.name, summary, text.lines().last().unwrap_or("")));
        }
        let code = o.status.code().unwrap_or(-1);
        if code != h.exit {
            return Some(format!("{}: exit {} (expected {})", h.name, code, h.exit));
        }
        return None;
    }
    let o = Command::new(WJ).args(["run", "--interpret"]).arg(path).output().unwrap();
    compare(h, &lines(&o), o.status.code().unwrap_or(-1))
}

#[test]
fn corpus_is_large_and_well_formed() {
    let ex = load_examples(&dir()).unwrap();
    assert!(ex.len() >= 150, "need >= 150 verified programs, have {}", ex.len());
    let mut problems = Vec::new();
    for (h, _) in &ex {
        if h.tags.is_empty() {
            problems.push(format!("{}: missing // tags:", h.name));
        }
        if h.is_test() {
            if h.tests_passed.is_none() {
                problems.push(format!("{}: test file needs // tests-passed:", h.name));
            }
        } else {
            if h.expect.is_empty() {
                problems.push(format!("{}: no // expect: lines (a program must have an oracle)", h.name));
            }
            if h.rust == RustStatus::NotApplicable {
                problems.push(format!("{}: missing // rust: ok|gap header", h.name));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let tags: std::collections::HashSet<_> = ex.iter().flat_map(|(h, _)| h.tags.clone()).collect();
    for needed in ["vec", "hashmap", "option", "result", "trait", "enum", "closures", "iterators", "pipe", "strings", "testing", "modules"] {
        assert!(tags.contains(needed), "corpus must cover tag `{}`", needed);
    }
}

#[test]
fn every_program_matches_its_oracle_in_the_interpreter() {
    let ex = load_examples(&dir()).unwrap();
    let items: Vec<ExampleHeader> = ex.into_iter().map(|(h, _)| h).collect();
    let failures = parallel(items, |h| {
        let p = dir().join(format!("{}.wj", h.name));
        run_example(&h, &p)
    });
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn every_program_is_lint_clean() {
    let ex = load_examples(&dir()).unwrap();
    let items: Vec<String> = ex.into_iter().map(|(h, _)| h.name).collect();
    let failures = parallel(items, |name| {
        let p = dir().join(format!("{}.wj", name));
        let o = Command::new(WJ).args(["lint", "--strict"]).arg(&p).output().unwrap();
        if o.status.success() {
            None
        } else {
            Some(format!("{}: lint: {}", name, String::from_utf8_lossy(&o.stdout).lines().take(3).collect::<Vec<_>>().join(" | ")))
        }
    });
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `wj build --no-cargo` + `rustc`; None when rustc is unavailable.
fn compiled(h: &ExampleHeader, path: &Path) -> Result<Vec<String>, String> {
    let d = TempDir::new().unwrap();
    let f = d.path().join("test.wj");
    fs::copy(path, &f).unwrap();
    fs::create_dir_all(d.path().join("build")).unwrap();
    let b = Command::new(WJ).args(["build", "--target", "rust", "--no-cargo"]).arg(&f).current_dir(d.path()).output().unwrap();
    if !b.status.success() {
        return Err(format!("{}: wj build failed: {}", h.name, String::from_utf8_lossy(&b.stderr).chars().take(300).collect::<String>()));
    }
    let bin = d.path().join("bin");
    let r = Command::new("rustc")
        .arg(d.path().join("build/test.rs"))
        .arg("-o")
        .arg(&bin)
        .args(["--edition", "2021", "-A", "warnings"])
        .output()
        .unwrap();
    if !r.status.success() {
        return Err(format!("{}: rustc failed: {}", h.name, String::from_utf8_lossy(&r.stderr).chars().take(300).collect::<String>()));
    }
    let o = Command::new(&bin).output().unwrap();
    Ok(lines(&o))
}

/// Ratchet: the compiled-path gap count only shrinks. Lower this when a gap is fixed; never raise it.
/// North star: zero (`// rust: gap` must disappear from the corpus).
const MAX_RUST_GAPS: usize = 55;

#[test]
fn rust_gap_count_only_shrinks() {
    let all = load_examples(&dir()).unwrap();
    let gaps = all.iter().filter(|(h, _)| matches!(h.rust, RustStatus::Gap(_))).count();
    assert!(gaps <= MAX_RUST_GAPS, "{gaps} gap programs > ratchet {MAX_RUST_GAPS}: a regression moved a program back to `rust: gap`");
    assert!(
        gaps == MAX_RUST_GAPS,
        "{gaps} gap programs < ratchet {MAX_RUST_GAPS}: lower MAX_RUST_GAPS to {gaps} to lock in the progress"
    );
}

#[test]
fn rust_ok_programs_compile_and_match() {
    if Command::new("rustc").arg("--version").output().map(|o| !o.status.success()).unwrap_or(true) {
        eprintln!("rustc not available: skipping");
        return;
    }
    let all = load_examples(&dir()).unwrap();
    let items: Vec<ExampleHeader> = all.into_iter().map(|(h, _)| h).filter(|h| h.rust == RustStatus::Ok).collect();
    assert!(items.len() >= 50, "expected a substantial compiled-path corpus, have {}", items.len());
    let failures = parallel(items, |h| {
        let p = dir().join(format!("{}.wj", h.name));
        match compiled(&h, &p) {
            Ok(out) => compare(&h, &out, h.exit),
            Err(e) => Some(e),
        }
    });
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

/// Opt-in ratchet: `WJ_VERIFY_GAPS=1` lists `// rust: gap` programs that now compile and match, so their
/// header can be promoted to `// rust: ok`.
#[test]
fn rust_gap_programs_are_reported_when_fixed() {
    if std::env::var("WJ_VERIFY_GAPS").is_err() {
        return;
    }
    let all = load_examples(&dir()).unwrap();
    let items: Vec<ExampleHeader> = all.into_iter().map(|(h, _)| h).filter(|h| matches!(h.rust, RustStatus::Gap(_))).collect();
    let fixed = parallel(items, |h| {
        let p = dir().join(format!("{}.wj", h.name));
        match compiled(&h, &p) {
            Ok(out) if compare(&h, &out, h.exit).is_none() => Some(format!("{} now compiles: change its header to `// rust: ok`", h.name)),
            _ => None,
        }
    });
    for f in &fixed {
        eprintln!("{}", f);
    }
}

#[test]
fn harness_rejects_wrong_output() {
    let h = windjammer::agent_index::verified::parse_header("x", "// tags: a\n// expect: 1\n// expect: 2\n");
    assert!(compare(&h, &["1".into(), "2".into()], 0).is_none());
    assert!(compare(&h, &["1".into(), "3".into()], 0).is_some(), "wrong line must fail");
    assert!(compare(&h, &["1".into()], 0).is_some(), "missing line must fail");
    assert!(compare(&h, &["1".into(), "2".into()], 101).is_some(), "wrong exit must fail");
}

#[test]
fn manifest_and_docs_index_are_fresh() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = windjammer::agent_index::verified::build_manifest(&dir()).unwrap();
    let committed: serde_json::Value = serde_json::from_str(&fs::read_to_string(dir().join("manifest.json")).expect("manifest.json: run `wj agent-index`")).unwrap();
    assert_eq!(manifest, committed, "examples/verified/manifest.json is stale: run `wj agent-index`");
    let index = serde_json::to_value(windjammer::agent_index::docs_search::build_index(root).unwrap()).unwrap();
    let committed: serde_json::Value = serde_json::from_str(&fs::read_to_string(root.join("agent_index/docs_index.json")).unwrap()).unwrap();
    assert_eq!(index, committed, "agent_index/docs_index.json is stale: run `wj agent-index`");
}
