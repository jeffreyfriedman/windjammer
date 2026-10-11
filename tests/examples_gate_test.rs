#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! Every program under `examples/` (outside `examples/verified/`, which `verified_examples_test.rs` owns) is
//! exercised by CI or it does not exist. `examples/EXAMPLES.toml` assigns each file a mode:
//!
//! * `run`   the interpreter runs it (`wj run --interpret`); its `// expect: line` headers (an oracle written by hand,
//!           never copied from the interpreter) must equal stdout and `// exit: N` the exit status (default 0)
//! * `test`  `wj test --interpret` on the file must pass
//! * `check` it parses and passes the analyzer in-process (for programs that need a network, a database or a UI,
//!           and so cannot run in CI); a `reason` is required
//!
//! The manifest itself is checked: every example is covered by exactly one entry, every entry matches a file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

#[derive(Debug, serde::Deserialize)]
struct Manifest {
    #[serde(default)]
    example: Vec<Entry>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct Entry {
    glob: String,
    mode: String,
    #[serde(default)]
    reason: String,
    /// Files matching `glob` that belong to another entry.
    #[serde(default)]
    exclude: Vec<String>,
}

impl Entry {
    fn matches(&self, file: &str) -> bool {
        glob_match(&self.glob, file) && !self.exclude.iter().any(|x| x == file)
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn walk(dir: &Path, out: &mut Vec<String>, base: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out, base);
        } else if p.extension().is_some_and(|x| x == "wj") {
            out.push(p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Programs governed by this gate.
fn examples() -> Vec<String> {
    let mut all = Vec::new();
    walk(&root().join("examples"), &mut all, &root());
    all.into_iter().filter(|f| !f.starts_with("examples/verified/")).collect()
}

fn glob_match(pat: &str, text: &str) -> bool {
    fn go(p: &[u8], t: &[u8]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some(b'*') if p.get(1) == Some(&b'*') => {
                let rest = &p[2..];
                let rest = rest.strip_prefix(b"/").unwrap_or(rest);
                (0..=t.len()).any(|i| go(rest, &t[i..]))
            }
            Some(b'*') => (0..=t.len()).take_while(|&i| i == 0 || t[i - 1] != b'/').any(|i| go(&p[1..], &t[i..])),
            Some(c) => t.first() == Some(c) && go(&p[1..], &t[1..]),
        }
    }
    go(pat.as_bytes(), text.as_bytes())
}

/// Problems with the manifest against a list of example files (empty when sound).
fn manifest_problems(m: &Manifest, files: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for e in &m.example {
        if !["run", "test", "check"].contains(&e.mode.as_str()) {
            out.push(format!("`{}`: unknown mode `{}`", e.glob, e.mode));
        }
        if e.mode == "check" && e.reason.trim().is_empty() {
            out.push(format!("`{}`: mode check needs a reason (why can it not run in CI?)", e.glob));
        }
        if !files.iter().any(|f| e.matches(f)) {
            out.push(format!("`{}` matches no example (remove the entry)", e.glob));
        }
    }
    for f in files {
        let n = m.example.iter().filter(|e| e.matches(f)).count();
        if n == 0 {
            out.push(format!("{f}: not covered by examples/EXAMPLES.toml (add it, or delete the example)"));
        } else if n > 1 {
            out.push(format!("{f}: covered by {n} entries"));
        }
    }
    out
}

fn manifest() -> Manifest {
    let text = std::fs::read_to_string(root().join("examples/EXAMPLES.toml")).expect("examples/EXAMPLES.toml");
    toml::from_str(&text).expect("EXAMPLES.toml parses")
}

fn headers(src: &str) -> (Vec<String>, i32) {
    let expect = src.lines().filter_map(|l| l.trim().strip_prefix("// expect:")).map(|e| e.strip_prefix(' ').unwrap_or(e).trim_end().to_string()).collect();
    let exit = src.lines().find_map(|l| l.trim().strip_prefix("// exit:")).and_then(|e| e.trim().parse().ok()).unwrap_or(0);
    (expect, exit)
}

fn check_in_process(src: String) -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            std::panic::catch_unwind(|| {
                let tokens = windjammer::lexer::Lexer::new(&src).tokenize_with_locations();
                let program = windjammer::parser::Parser::new_with_source(tokens, "example.wj".into(), src.clone()).parse().map_err(|e| format!("parse error: {e}"))?;
                windjammer::analyzer::Analyzer::new().analyze_program(&program).map(|_| ()).map_err(|e| format!("analyzer: {e}"))
            })
            .unwrap_or_else(|_| Err("panicked".into()))
        })
        .unwrap()
        .join()
        .unwrap_or_else(|_| Err("thread panicked".into()))
}

fn run_one(file: &str, e: &Entry) -> Option<String> {
    let path = root().join(file);
    let src = std::fs::read_to_string(&path).unwrap_or_default();
    match e.mode.as_str() {
        "check" => check_in_process(src).err().map(|m| format!("{file}: check failed: {m}")),
        "run" => {
            let (expect, exit) = headers(&src);
            if expect.is_empty() && exit == 0 {
                return Some(format!("{file}: mode run needs `// expect:` lines (an independent oracle); use mode check if it has no deterministic output"));
            }
            let o = Command::new(WJ).args(["run", "--interpret"]).arg(&path).output().unwrap();
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            let got: Vec<String> = text.lines().filter(|l| !l.starts_with("Interpreting ")).map(|l| l.trim_end().to_string()).collect();
            let code = o.status.code().unwrap_or(-1);
            if code != exit {
                return Some(format!("{file}: exit {code}, expected {exit}: {}", String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
            }
            (got != expect).then(|| format!("{file}: expected {expect:?}, got {got:?}"))
        }
        "test" => {
            let o = Command::new(WJ).args(["test", "--interpret"]).arg(&path).output().unwrap();
            (!o.status.success()).then(|| format!("{file}: wj test --interpret failed: {}", String::from_utf8_lossy(&o.stdout).lines().last().unwrap_or("")))
        }
        _ => None,
    }
}

fn run_all(files: Vec<(String, Entry)>) -> Vec<String> {
    use std::sync::{Arc, Mutex};
    let queue = Arc::new(Mutex::new(files));
    let failures = Arc::new(Mutex::new(Vec::new()));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let (q, f) = (queue.clone(), failures.clone());
            std::thread::spawn(move || loop {
                let item = q.lock().unwrap().pop();
                match item {
                    Some((file, e)) => {
                        if let Some(m) = run_one(&file, &e) {
                            f.lock().unwrap().push(m);
                        }
                    }
                    None => break,
                }
            })
        })
        .collect();
    workers.into_iter().for_each(|w| w.join().unwrap());
    let mut v = failures.lock().unwrap().clone();
    v.sort();
    v
}

#[test]
fn every_example_is_covered_by_the_manifest() {
    let p = manifest_problems(&manifest(), &examples());
    assert!(p.is_empty(), "examples/EXAMPLES.toml:\n{}", p.join("\n"));
}

#[test]
fn every_example_passes_its_mode() {
    let m = manifest();
    let files: Vec<(String, Entry)> = examples().into_iter().filter_map(|f| m.example.iter().find(|e| e.matches(&f)).cloned().map(|e| (f, e))).collect();
    assert!(!files.is_empty());
    let bad = run_all(files);
    assert!(bad.is_empty(), "examples that CI cannot run or check:\n{}", bad.join("\n"));
}

/// Mutation self-tests of the gate itself.
#[test]
fn gate_detects_uncovered_stale_and_broken_examples() {
    let files = vec!["examples/a.wj".to_string(), "examples/b/c.wj".to_string()];
    let m = |toml: &str| -> Manifest { toml::from_str(toml).unwrap() };
    let ok = m("[[example]]\nglob = \"examples/a.wj\"\nmode = \"run\"\n[[example]]\nglob = \"examples/b/*.wj\"\nmode = \"check\"\nreason = \"needs network\"\n");
    assert!(manifest_problems(&ok, &files).is_empty());
    let uncovered = m("[[example]]\nglob = \"examples/a.wj\"\nmode = \"run\"\n");
    assert!(manifest_problems(&uncovered, &files).iter().any(|p| p.contains("not covered")));
    let stale = m("[[example]]\nglob = \"examples/gone.wj\"\nmode = \"run\"\n[[example]]\nglob = \"examples/**/*.wj\"\nmode = \"run\"\n");
    assert!(manifest_problems(&stale, &files).iter().any(|p| p.contains("matches no example")));
    let no_reason = m("[[example]]\nglob = \"examples/**/*.wj\"\nmode = \"check\"\n");
    assert!(manifest_problems(&no_reason, &files).iter().any(|p| p.contains("needs a reason")));
    let twice = m("[[example]]\nglob = \"examples/a.wj\"\nmode = \"run\"\n[[example]]\nglob = \"examples/**/*.wj\"\nmode = \"run\"\n");
    assert!(manifest_problems(&twice, &files).iter().any(|p| p.contains("covered by 2")));
    // a broken program fails the check mode; a sound one passes
    assert!(check_in_process("fn ( {{ broken".into()).is_err());
    assert!(check_in_process("fn main() {\n    println(\"hi\")\n}\n".into()).is_ok());
    assert_eq!(headers("// expect: a\n// expect: b\n// exit: 3\nfn main() {}\n"), (vec!["a".to_string(), "b".to_string()], 3));
}

#[allow(dead_code)]
fn _unused(_: BTreeMap<u8, u8>) {}
