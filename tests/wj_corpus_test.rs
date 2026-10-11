//! Compile-checks every standalone `.wj` corpus file under `tests/`.
//!
//! These files used to have no consumer (their shell runners were never wired into CI). Each
//! one must go through the front end (lex, parse, analyze, codegen) without a diagnostic or a
//! panic. Rust-level expectations (rustc error translation, program output) are NOT asserted here;
//! `tests/conformance/run_conformance_tests.sh` is the manual end-to-end runner for those.
//!
//! Files that currently fail are listed in `tests/wj_corpus_known_failures.txt` as
//! `<path> | <one-line cause>`. A listed file that starts compiling fails the run until its entry
//! is removed, so the list can only shrink.
//!
//! consumes: tests/*.wj
//! consumes: tests/auto_clone/*.wj
//! consumes: tests/conformance/*.wj
//! consumes: tests/conformance/*/*.wj
//! consumes: tests/windjammer_tests/*.wj
//! consumes: tests/integration_wj/test_cases/*.wj
//! consumes: tests/error_system/*.wj
//! consumes: tests/error_system_e2e/*.wj
//! consumes: tests/fixtures/*.wj
//! consumes: tests/fixtures/traits/*.wj
//! consumes: tests/fixtures/library_multipass/*.wj
//! consumes: tests/dogfood/*.wj

use std::collections::BTreeMap;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use windjammer::{build_project, CompilationTarget};

const KNOWN_FAILURES: &str = "tests/wj_corpus_known_failures.txt";

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn known_failures() -> BTreeMap<String, String> {
    let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(KNOWN_FAILURES))
        .unwrap_or_default();
    let mut map = BTreeMap::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let (path, cause) = line
            .split_once('|')
            .unwrap_or_else(|| panic!("{KNOWN_FAILURES}: entry without a cause: {line}"));
        assert!(!cause.trim().is_empty(), "{KNOWN_FAILURES}: empty cause for {path}");
        map.insert(path.trim().to_string(), cause.trim().to_string());
    }
    map
}

fn wj_files_in(dir: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(tests_dir().join(dir))
        .unwrap_or_else(|e| panic!("read tests/{dir}: {e}"))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "wj"))
        .collect();
    files.sort();
    files
}

/// `Ok(())` when the front end accepts the file; `Err(first line of the problem)` otherwise.
fn compile(file: &Path) -> Result<(), String> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let source = tmp.path().join(file.file_name().unwrap());
    fs::copy(file, &source).expect("copy corpus file");
    let out = tmp.path().join("out");
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        build_project(&source, &out, CompilationTarget::Rust, true)
    }));
    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e.to_string().lines().next().unwrap_or("error").to_string()),
        Err(_) => Err("compiler panicked".to_string()),
    }
}

fn check_dir(dir: &str) {
    let known = known_failures();
    let mut unexpected = Vec::new();
    let mut fixed = Vec::new();
    for file in wj_files_in(dir) {
        let rel = format!("tests/{dir}/{}", file.file_name().unwrap().to_string_lossy());
        let rel = rel.replace("tests//", "tests/");
        match (compile(&file), known.contains_key(&rel)) {
            (Err(e), false) => unexpected.push(format!("  {rel}: {e}")),
            (Ok(()), true) => fixed.push(format!("  {rel}")),
            _ => {}
        }
    }
    assert!(
        unexpected.is_empty(),
        "corpus files that no longer compile (fix them, or list them with a cause in \
         {KNOWN_FAILURES}):\n{}",
        unexpected.join("\n")
    );
    assert!(
        fixed.is_empty(),
        "corpus files listed in {KNOWN_FAILURES} now compile; remove their entries:\n{}",
        fixed.join("\n")
    );
}

#[test]
fn corpus_top_level() {
    check_dir("");
}

#[test]
fn corpus_conformance() {
    check_dir("conformance");
    for sub in ["basics", "control_flow", "error_handling", "stdlib", "types", "values"] {
        check_dir(&format!("conformance/{sub}"));
    }
}

#[test]
fn corpus_error_system() {
    check_dir("error_system");
    check_dir("error_system_e2e");
}

#[test]
fn corpus_regression_dirs() {
    for dir in ["auto_clone", "windjammer_tests", "integration_wj/test_cases", "dogfood"] {
        check_dir(dir);
    }
}

#[test]
fn corpus_fixtures() {
    check_dir("fixtures");
    check_dir("fixtures/traits");
    check_dir("fixtures/library_multipass");
}

#[test]
fn known_failure_entries_point_at_existing_files() {
    for path in known_failures().keys() {
        assert!(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(path).is_file(),
            "{KNOWN_FAILURES}: {path} does not exist"
        );
    }
}
