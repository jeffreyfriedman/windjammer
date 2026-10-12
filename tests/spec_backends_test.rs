// consumes: tests/spec_backend_status.tsv
//! Language-spec conformance gate: every runnable example in tests/spec/** (rule-tagged, with an
//! `// expect:` oracle written from the spec, not from any backend) runs on the interpreter, Rust,
//! JS (default) and Go, TypeScript (WJ_SPEC_FULL=1), through the shared conformance harness.
//!
//! Status per example and backend: PASS; WRONG = ran but printed the wrong output or exit status
//! (a silent miscompile: the loudest category); REJECTED = wj or the toolchain refused it.
//! Ratchet: tests/spec_backend_status.tsv. Any change fails: regressions (and every WRONG not
//! already recorded) with their diff; improvements until recorded with SPEC_BLESS=1 (which refuses
//! to write while anything regressed). `// xfail: Q-NNN` must name a `## Q-NNN` in
//! docs/spec/OPEN_QUESTIONS.md. Until the spec corpus is in the tree the gate reports and passes;
//! `WJ_SPEC_ROOT=<checkout>` points it at another checkout.

#![allow(clippy::duplicate_mod)]

#[path = "common/conformance/mod.rs"]
mod conformance;

use conformance::backends::{compile_and_run, run_interpreter, Backend, Outcome};
use conformance::toolchain::Tool;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const WJ: &str = env!("CARGO_BIN_EXE_wj");
const COLUMNS: [&str; 5] = ["interp", "rust", "go", "js", "ts"];
const STATUS_FILE: &str = "tests/spec_backend_status.tsv";

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn spec_root() -> PathBuf {
    std::env::var("WJ_SPEC_ROOT").map(PathBuf::from).unwrap_or_else(|_| repo())
}

#[derive(Debug, Clone)]
struct Example {
    id: String,
    source: String,
    mode: String,
    expect: String,
    xfail: Option<String>,
}

fn header(src: &str, key: &str) -> Option<String> {
    src.lines().find_map(|l| l.strip_prefix(&format!("// {key}:")).map(|v| v.trim().to_string()))
}

fn load(root: &Path) -> Vec<Example> {
    let mut files = Vec::new();
    fn walk(d: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out)
            } else if p.extension().is_some_and(|x| x == "wj") {
                out.push(p)
            }
        }
    }
    walk(&root.join("tests/spec"), &mut files);
    files.sort();
    files
        .into_iter()
        .map(|f| {
            let source = std::fs::read_to_string(&f).unwrap();
            let expect: Vec<String> = source
                .lines()
                .filter_map(|l| l.strip_prefix("// expect:").map(|v| v.strip_prefix(' ').unwrap_or(v).to_string()))
                .collect();
            Example {
                // One row per file: several files may demonstrate the same rule.
                id: f.strip_prefix(root.join("tests/spec")).unwrap().with_extension("").to_string_lossy().into_owned(),
                mode: header(&source, "mode").unwrap_or_else(|| "run".into()),
                xfail: header(&source, "xfail").map(|x| x.split_whitespace().next().unwrap_or("").to_string()),
                expect: expect.iter().map(|l| format!("{l}\n")).collect(),
                source,
            }
        })
        .filter(|e| !matches!(e.mode.as_str(), "parse-ok" | "parse-error"))
        .collect()
}

/// `PASS` / `WRONG` / `REJECTED` from an outcome (`stdout`, exit code), judged by the spec oracle.
fn judge(ex: &Example, outcome: Option<(String, i32)>) -> &'static str {
    let Some((stdout, code)) = outcome else { return "REJECTED" };
    let exit_ok = if ex.mode == "panic" { code != 0 } else { code == 0 };
    if exit_ok && stdout == ex.expect {
        "PASS"
    } else {
        "WRONG"
    }
}

fn run(ex: &Example, col: &str) -> &'static str {
    let tmp = tempfile::TempDir::new().unwrap();
    let wj = Path::new(WJ);
    let o = match col {
        "interp" => run_interpreter(wj, tmp.path(), &ex.source),
        "rust" => compile_and_run(wj, Backend::Rust, tmp.path(), &ex.source),
        "go" => compile_and_run(wj, Backend::Go, tmp.path(), &ex.source),
        "js" => compile_and_run(wj, Backend::Js, tmp.path(), &ex.source),
        _ => compile_and_run(wj, Backend::Ts, tmp.path(), &ex.source),
    };
    match o {
        // The interpreter reports a front-end rejection as a non-zero exit before printing anything.
        Outcome::Ran(_, e) if col == "interp" && e.code != 0 && e.stdout.is_empty() && ex.mode != "panic" => judge(ex, None),
        Outcome::Ran(_, e) if !e.timed_out => judge(ex, Some((e.stdout, e.code))),
        _ => judge(ex, None),
    }
}

fn active_columns() -> Vec<&'static str> {
    let full = std::env::var("WJ_SPEC_FULL").is_ok();
    COLUMNS
        .iter()
        .copied()
        .filter(|c| match *c {
            "interp" => true,
            "rust" => Tool::Rustc.available(),
            "js" => Tool::Node.available(),
            "go" => full && Tool::Go.available(),
            _ => full && Tool::Node.available(),
        })
        .collect()
}

type Table = BTreeMap<String, BTreeMap<String, String>>;

fn read_status(path: &Path) -> Table {
    let mut t = Table::new();
    let text = std::fs::read_to_string(path).unwrap_or_default();
    for l in text.lines().skip(1).filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let cells: Vec<&str> = l.split('\t').collect();
        let row = t.entry(cells[0].to_string()).or_default();
        for (c, v) in COLUMNS.iter().zip(&cells[1..]) {
            row.insert(c.to_string(), v.to_string());
        }
    }
    t
}

fn write_status(path: &Path, t: &Table, xfails: &BTreeMap<String, String>) {
    let mut s = format!("example\t{}\txfail\n", COLUMNS.join("\t"));
    for (id, row) in t {
        let cells: Vec<String> = COLUMNS.iter().map(|c| row.get(*c).cloned().unwrap_or_else(|| "-".into())).collect();
        s.push_str(&format!("{id}\t{}\t{}\n", cells.join("\t"), xfails.get(id).map(String::as_str).unwrap_or("-")));
    }
    std::fs::write(path, s).unwrap();
}

fn rank(s: &str) -> u8 {
    match s {
        "PASS" => 2,
        "REJECTED" => 1,
        _ => 0, // WRONG is worse than refusing: it silently computes something else
    }
}

#[test]
fn spec_examples_conform_on_every_backend() {
    let root = spec_root();
    let examples = load(&root);
    if examples.is_empty() {
        eprintln!("spec backends: no tests/spec corpus in {} (lands with wt/lang-spec); nothing to check", root.display());
        assert!(!repo().join("docs/spec/README.md").exists(), "docs/spec exists but tests/spec has no runnable examples");
        return;
    }
    let questions = std::fs::read_to_string(root.join("docs/spec/OPEN_QUESTIONS.md")).unwrap_or_default();
    let bad_xfail: Vec<_> = examples.iter().filter_map(|e| e.xfail.as_ref().filter(|q| !questions.contains(&format!("## {q}"))).map(|q| format!("{}: xfail {q}", e.id))).collect();
    assert!(bad_xfail.is_empty(), "xfails must name an entry of docs/spec/OPEN_QUESTIONS.md: {bad_xfail:?}");

    let cols = active_columns();
    let jobs: Vec<(usize, &str)> = (0..examples.len()).flat_map(|i| cols.iter().map(move |c| (i, *c))).collect();
    let results = std::sync::Mutex::new(Table::new());
    std::thread::scope(|s| {
        for chunk in jobs.chunks(jobs.len().div_ceil(4).max(1)) {
            let (results, examples) = (&results, &examples);
            s.spawn(move || {
                for (i, c) in chunk {
                    let st = run(&examples[*i], c);
                    results.lock().unwrap().entry(examples[*i].id.clone()).or_default().insert(c.to_string(), st.to_string());
                }
            });
        }
    });
    let got = results.into_inner().unwrap();
    let path = repo().join(STATUS_FILE);
    let was = read_status(&path);
    let (mut wrong, mut worse, mut better) = (Vec::new(), Vec::new(), Vec::new());
    for (id, row) in &got {
        for (c, st) in row {
            let old = was.get(id).and_then(|r| r.get(c)).cloned().unwrap_or_else(|| "NEW".into());
            if st == &old {
                continue;
            }
            let line = format!("{id} [{c}]: {old} -> {st}");
            if st == "WRONG" {
                wrong.push(line);
            } else if old == "NEW" || rank(st) < rank(&old) {
                worse.push(line);
            } else {
                better.push(line);
            }
        }
    }
    let totals: Vec<String> = cols
        .iter()
        .map(|c| {
            let n = |s: &str| got.values().filter(|r| r.get(*c).map(String::as_str) == Some(s)).count();
            format!("{c}: {} pass / {} WRONG / {} rejected", n("PASS"), n("WRONG"), n("REJECTED"))
        })
        .collect();
    eprintln!("spec backends ({} examples): {}", examples.len(), totals.join("; "));
    if wrong.is_empty() && worse.is_empty() && std::env::var("SPEC_BLESS").is_ok() {
        let mut merged = was.clone();
        for (id, row) in &got {
            merged.entry(id.clone()).or_default().extend(row.clone());
        }
        merged.retain(|id, _| got.contains_key(id));
        let xf = examples.iter().filter_map(|e| e.xfail.clone().map(|q| (e.id.clone(), q))).collect();
        write_status(&path, &merged, &xf);
        return;
    }
    let mut msg = String::new();
    if !wrong.is_empty() {
        msg.push_str(&format!("WRONG OUTPUT (backend ran and printed something the spec forbids):\n  {}\n", wrong.join("\n  ")));
    }
    if !worse.is_empty() {
        msg.push_str(&format!("regressions:\n  {}\n", worse.join("\n  ")));
    }
    if !better.is_empty() {
        msg.push_str(&format!("improvements (record with SPEC_BLESS=1):\n  {}\n", better.join("\n  ")));
    }
    assert!(msg.is_empty(), "{msg}");
}

#[test]
fn selftest_judge_separates_wrong_from_rejected() {
    let ex = Example { id: "X".into(), source: String::new(), mode: "run".into(), expect: "3\n".into(), xfail: None };
    assert_eq!(judge(&ex, Some(("3\n".into(), 0))), "PASS");
    assert_eq!(judge(&ex, Some(("4\n".into(), 0))), "WRONG");
    assert_eq!(judge(&ex, Some(("3\n".into(), 1))), "WRONG");
    assert_eq!(judge(&ex, None), "REJECTED");
    let p = Example { mode: "panic".into(), ..ex };
    assert_eq!(judge(&p, Some(("3\n".into(), 101))), "PASS");
    assert_eq!(judge(&p, Some(("3\n".into(), 0))), "WRONG", "a panic example that exits 0 is wrong");
    assert!(rank("WRONG") < rank("REJECTED") && rank("REJECTED") < rank("PASS"));
}
