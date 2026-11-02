//! Project gates: the convergence gates (docs/spec/convergence-plan.md, C0) for multi-file projects, which go
//! through the library multipass build (`compiler::library_multipass`) that single-file goldens never reach.
//! Every project in `tests/project_corpus/<name>/` (a `main.wj` plus modules) is built with
//! `wj build <dir> --no-cargo` and judged three ways:
//!
//!   1. **Golden emitted Rust**: every generated `.rs` file, sorted and concatenated, equals
//!      `tests/golden/projects/<name>.golden`. Refresh with `UPDATE_GOLDEN=1` and review the diff.
//!   2. **rustc accepts the crate** (independent oracle): `rustc main.rs` (or `lib.rs`) on the output.
//!   3. **Interpreter vs compiled**: the compiled program prints the `// expect:` lines of `main.wj`
//!      (computed by hand) and the same text as `wj run main.wj`.
//!
//! Projects that fail a stage are listed in `tests/golden/project_rejects.txt` with the stage; the list may
//! only shrink (`UPDATE_GOLDEN=1` drops entries that pass; adding one needs `WJ_GATE_BASELINE=1` and a
//! reviewed reason). Projects that need `windjammer_runtime` are recorded as `runtime`, not compiled.
//!
//! Standalone: `cargo test --release --test project_gates`. `WJ_GATE_FILTER=<substring>` restricts the corpus.
//!
//! consumes: tests/project_corpus/**
//! consumes: tests/golden/projects/**

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const CORPUS: &str = "tests/project_corpus";
const GOLDEN_DIR: &str = "tests/golden/projects";
const REJECTS: &str = "tests/golden/project_rejects.txt";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wj() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn updating() -> bool {
    std::env::var("UPDATE_GOLDEN").is_ok_and(|v| v == "1")
}

fn baseline() -> bool {
    std::env::var("WJ_GATE_BASELINE").is_ok_and(|v| v == "1")
}

fn filter() -> Option<String> {
    std::env::var("WJ_GATE_FILTER").ok().filter(|f| !f.is_empty())
}

fn projects() -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(root().join(CORPUS))
        .expect(CORPUS)
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("main.wj").is_file())
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    out.sort();
    if let Some(f) = filter() {
        out.retain(|p| p.contains(&f));
    }
    out
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else if p.extension().is_some_and(|x| x == "wj") {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

fn rs_files(dir: &Path, base: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != "target") {
                rs_files(&p, base, out);
            }
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p.strip_prefix(base).unwrap().display().to_string());
        }
    }
}

fn expected(main_src: &str) -> String {
    main_src
        .lines()
        .filter_map(|l| l.trim().strip_prefix("// expect:"))
        .map(|l| format!("{}\n", l.trim()))
        .collect()
}

/// What one project produced: the golden text and the first failing stage, if any.
struct Outcome {
    golden: String,
    failure: Option<String>,
}

fn first_error(stderr: &[u8]) -> String {
    let err = String::from_utf8_lossy(stderr);
    let line = err.lines().find(|l| l.starts_with("error")).unwrap_or("error");
    line.chars().take(160).collect()
}

fn judge(name: &str) -> Outcome {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    copy_dir(&root().join(CORPUS).join(name), &src);
    let out = tmp.path().join("out");
    let build = Command::new(wj())
        .arg("build")
        .arg(&src)
        .args(["--target", "rust", "--no-cargo", "--no-lint", "--quiet", "-o"])
        .arg(&out)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let mut files = Vec::new();
    rs_files(&out, &out, &mut files);
    files.sort();
    if !build.status.success() || files.is_empty() {
        return Outcome { golden: "// wj build failed\n".into(), failure: Some(format!("build: {}", first_error(&build.stderr))) };
    }
    let mut golden = String::new();
    for f in &files {
        golden.push_str(&format!("// ==== {f} ====\n"));
        golden.push_str(&std::fs::read_to_string(out.join(f)).unwrap_or_default());
    }
    if golden.contains("windjammer_runtime") {
        return Outcome { golden, failure: Some("runtime: needs windjammer_runtime (not compiled by this gate)".into()) };
    }
    let (crate_file, crate_type) = if out.join("main.rs").is_file() { ("main.rs", "bin") } else { ("lib.rs", "lib") };
    let exe = tmp.path().join("prog");
    let rustc = Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", crate_type, "-A", "warnings", "--crate-name", "project", "-o"])
        .arg(&exe)
        .arg(out.join(crate_file))
        .output()
        .unwrap();
    if !rustc.status.success() {
        return Outcome { golden, failure: Some(format!("rustc: {}", first_error(&rustc.stderr))) };
    }
    if crate_type == "lib" {
        return Outcome { golden, failure: None };
    }
    let main_src = std::fs::read_to_string(src.join("main.wj")).unwrap();
    let compiled = String::from_utf8_lossy(&Command::new(&exe).output().unwrap().stdout).into_owned();
    let interp = Command::new(wj()).arg("run").arg(src.join("main.wj")).current_dir(&src).output().unwrap();
    let interp = String::from_utf8_lossy(&interp.stdout).into_owned();
    let want = expected(&main_src);
    let failure = if !want.is_empty() && compiled != want {
        Some(format!("output: compiled printed {compiled:?}, expected {want:?}"))
    } else if compiled != interp {
        Some(format!("output: compiled printed {compiled:?}, interpreter {interp:?}"))
    } else {
        None
    };
    Outcome { golden, failure }
}

fn read_rejects() -> BTreeMap<String, String> {
    std::fs::read_to_string(root().join(REJECTS))
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once(" | ").map(|(a, b)| (a.trim().to_string(), b.trim().to_string())))
        .collect()
}

fn write_rejects(map: &BTreeMap<String, String>) {
    let mut s = String::from(
        "# Multi-file projects that fail a project gate (tests/project_gates_test.rs). May only shrink.\n\
         # <project> | <stage>: <first error>   (stages: build, runtime, rustc, output)\n",
    );
    for (k, v) in map {
        s.push_str(&format!("{k} | {v}\n"));
    }
    std::fs::write(root().join(REJECTS), s).unwrap();
}

#[test]
fn project_goldens_rustc_and_interpreter_agree() {
    let names = projects();
    assert!(!names.is_empty(), "{CORPUS} has no projects");
    let known = read_rejects();
    let mut diffs = Vec::new();
    let mut newly_failing = Vec::new();
    let mut now_passing = Vec::new();
    let mut current = BTreeMap::new();
    for name in &names {
        let o = judge(name);
        let gp = root().join(GOLDEN_DIR).join(format!("{name}.golden"));
        if std::fs::read_to_string(&gp).ok().as_deref() != Some(o.golden.as_str()) {
            if updating() {
                std::fs::create_dir_all(gp.parent().unwrap()).unwrap();
                std::fs::write(&gp, &o.golden).unwrap();
            }
            diffs.push(name.clone());
        }
        match (&o.failure, known.get(name)) {
            (Some(f), None) => newly_failing.push(format!("  {name}: {f}")),
            (Some(f), Some(k)) if f.split(':').next() != k.split(':').next() => {
                newly_failing.push(format!("  {name}: {f} (listed as {k})"))
            }
            (None, Some(_)) => now_passing.push(format!("  {name}")),
            _ => {}
        }
        if let Some(f) = o.failure {
            current.insert(name.clone(), f);
        }
    }
    if updating() && filter().is_none() && (newly_failing.is_empty() || baseline()) {
        write_rejects(&current);
        if baseline() {
            newly_failing.clear();
        }
    }
    let mut msg = String::new();
    if !diffs.is_empty() && !updating() {
        msg.push_str(&format!("{} project golden(s) differ (review, then UPDATE_GOLDEN=1):\n  {}\n", diffs.len(), diffs.join("\n  ")));
    }
    if !newly_failing.is_empty() {
        msg.push_str(&format!("{} project(s) newly fail (fix the compiler; do not list them):\n{}\n", newly_failing.len(), newly_failing.join("\n")));
    }
    if !now_passing.is_empty() && !updating() {
        msg.push_str(&format!("{} listed project(s) now pass: rerun with UPDATE_GOLDEN=1 to shrink {REJECTS}:\n{}\n", now_passing.len(), now_passing.join("\n")));
    }
    assert!(msg.is_empty(), "{msg}");
}
