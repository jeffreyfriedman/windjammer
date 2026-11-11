//! Precision gate for WJ0051 (ADR-0002), with rustc as the independent oracle: a binding WJ0051 reports must be one
//! the program really changes. Method: take a corpus program that declares its `mut`s, remove every `let mut` /
//! `for mut` (as if written before the migration), and collect the bindings WJ0051 reports. Then build the original
//! program to Rust and run rustc with `unused_mut` on: a reported binding that rustc says does not need `mut` is a
//! false positive (a binding the check claims is changed while the compiled program never changes it).
//!
//! The default test scans the verified examples; `--ignored` scans every single-file corpus (minutes). The oracle has a
//! self-test: a `mut` on a binding that is never changed must be flagged by it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wj_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            wj_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "wj") {
            out.push(p);
        }
    }
}

/// `src` as written before the migration: no `let mut` / `for mut`.
fn demut(src: &str) -> String {
    src.replace("let mut ", "let ").replace("for mut ", "for ")
}

/// Names of the bindings WJ0051 reports in `src` (alone, as one program).
fn reported(name: &str, src: &str) -> BTreeSet<String> {
    windjammer::check::local_mutability_sources(&[(name.to_string(), src.to_string())])
        .iter()
        .filter_map(|d| {
            let s = d.secondary_spans.first()?;
            src.get(s.byte_start..s.byte_end).map(str::to_string)
        })
        .collect()
}

/// rustc's `unused_mut` warnings on the Rust `wj build --no-cargo` emits for `path`: the names of bindings declared
/// `mut` that are never changed. `None` when the program does not build or rustc rejects it on its own.
fn rust_unused_mut(path: &Path) -> Option<BTreeSet<String>> {
    // built from a copy in a scratch directory: the build reads `.wj.meta` sidecars of every ancestor directory
    let out = tempfile::tempdir().unwrap();
    let src = out.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let copy = src.join(path.file_name()?);
    std::fs::copy(path, &copy).ok()?;
    windjammer::build_project(&copy, &out.path().join("out"), windjammer::CompilationTarget::Rust, false).ok()?;
    let out_dir = out.path().join("out");
    let out = &out_dir;
    let stem = path.file_stem()?.to_string_lossy().to_string();
    let rs = out.join(format!("{stem}.rs"));
    unused_mut_of_file(&rs)
}

fn unused_mut_of_file(rs: &Path) -> Option<BTreeSet<String>> {
    let rust = std::fs::read_to_string(rs).ok()?;
    let tmp = tempfile::tempdir().unwrap();
    let crate_type = if rust.contains("fn main(") { "bin" } else { "lib" };
    let o = std::process::Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", crate_type, "--emit=metadata", "-W", "unused_mut", "--crate-name", "prec", "-o"])
        .arg(tmp.path().join("out.rmeta"))
        .arg(rs)
        .output()
        .expect("rustc on PATH");
    if !o.status.success() {
        return None;
    }
    let stderr = String::from_utf8_lossy(&o.stderr);
    let mut names = BTreeSet::new();
    let mut in_unused = false;
    for l in stderr.lines() {
        if l.starts_with("warning: variable does not need to be mutable") {
            in_unused = true;
        } else if in_unused {
            // the snippet line `let mut name` (or `mut name` in a pattern)
            if let Some(i) = l.find("mut ") {
                let name: String = l[i + 4..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                if !name.is_empty() {
                    names.insert(name);
                    in_unused = false;
                }
            }
        }
    }
    Some(names)
}

/// file -> bindings WJ0051 reports that rustc says are never changed.
fn false_positives(files: &[PathBuf]) -> (BTreeMap<String, BTreeSet<String>>, usize, usize) {
    let (mut fps, mut judged, mut flagged) = (BTreeMap::new(), 0, 0);
    for f in files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        if !src.contains("mut ") || src.contains("// mode: parse-error") {
            continue;
        }
        let names = reported(&f.display().to_string(), &demut(&src));
        if names.is_empty() {
            continue;
        }
        let Some(unused) = rust_unused_mut(f) else { continue };
        judged += 1;
        flagged += names.len();
        // rustc names a binding, not a place: a name bound more than once is left out (one of its bindings may
        // need `mut` while another does not)
        let d = demut(&src);
        let bound_once = |n: &String| ["let ", "for "].iter().map(|k| d.matches(&format!("{k}{n} ")).count() + d.matches(&format!("{k}{n}:")).count()).sum::<usize>() <= 1;
        let bad: BTreeSet<String> = names.intersection(&unused).filter(|n| bound_once(n)).cloned().collect();
        if !bad.is_empty() {
            fps.insert(f.strip_prefix(root()).unwrap_or(f).display().to_string(), bad);
        }
    }
    (fps, judged, flagged)
}

#[test]
fn the_oracle_flags_a_mut_that_is_never_needed() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("m.wj");
    std::fs::write(&f, "fn main() {\n    let mut never = 1\n    let mut once = 1\n    once += 1\n    println(\"${never} ${once}\")\n}\n").unwrap();
    let unused = rust_unused_mut(&f).expect("builds and type-checks");
    assert!(unused.contains("never") && !unused.contains("once"), "{unused:?}");
    // and the demutted program is reported exactly for the binding that changes
    let src = std::fs::read_to_string(&f).unwrap();
    assert_eq!(reported("m.wj", &demut(&src)), BTreeSet::from(["once".to_string()]));
}

#[test]
fn wj0051_reports_only_changed_bindings_in_the_verified_examples() {
    let mut files = Vec::new();
    wj_files(&root().join("examples/verified"), &mut files);
    files.sort();
    let (fps, judged, flagged) = false_positives(&files);
    assert!(judged >= 30 && flagged >= 60, "corpus too small: {judged} programs, {flagged} reported bindings");
    assert!(fps.is_empty(), "WJ0051 reports bindings rustc says are never changed:\n{fps:#?}");
}

#[test]
#[ignore = "full precision scan over every single-file corpus (minutes): cargo test --release --test all local_mutability_precision -- --ignored"]
fn wj0051_reports_only_changed_bindings_in_every_single_file_corpus() {
    let mut files = Vec::new();
    for d in ["tests/drift_corpus", "tests/spec", "examples", "tests/conformance", "tests/windjammer_tests", "bench"] {
        wj_files(&root().join(d), &mut files);
    }
    if let Ok(rd) = std::fs::read_dir(root().join("tests")) {
        files.extend(rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wj")));
    }
    files.sort();
    files.dedup();
    let (fps, judged, flagged) = false_positives(&files);
    eprintln!("precision scan: {judged} programs judged, {flagged} reported bindings, {} with a false positive", fps.len());
    assert!(fps.is_empty(), "WJ0051 reports bindings rustc says are never changed:\n{fps:#?}");
}
