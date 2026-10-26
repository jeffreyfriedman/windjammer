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

//! A build must not depend on what an earlier build left behind. A file's own previous `.wj.meta`
//! sidecar is an output, never an input: cold (no `.wj-cache`) and warm (cache from the previous
//! build) builds of the same source must emit byte-identical Rust. Regression: `trait_basics.wj`
//! emitted `d.speak("Rex")` warm and `d.speak("Rex".to_string())` cold.

use std::path::{Path, PathBuf};
use std::process::Command;

fn corpus_sample() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/verified");
    let mut all: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "wj"))
        .collect();
    all.sort();
    let must = ["trait_basics.wj", "generics_struct.wj"];
    let mut out: Vec<PathBuf> = all
        .iter()
        .filter(|p| must.iter().any(|m| p.file_name().is_some_and(|n| n == *m)))
        .cloned()
        .collect();
    // A stable sample: every 12th program.
    out.extend(all.iter().step_by(12).cloned());
    out.sort();
    out.dedup();
    out
}

fn build(project: &Path, src: &Path, out: &Path) -> Option<String> {
    let _ = std::fs::remove_dir_all(out);
    let o = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(project)
        .arg("build")
        .arg(src)
        .arg("-o")
        .arg(out)
        .arg("--no-cargo")
        .output()
        .unwrap();
    if !o.status.success() {
        return None;
    }
    let stem = src.file_stem().unwrap().to_string_lossy().to_string();
    std::fs::read_to_string(out.join(format!("{stem}.rs"))).ok()
}

#[test]
fn cold_and_warm_builds_emit_identical_rust() {
    let mut diverged = Vec::new();
    for file in corpus_sample() {
        let project = tempfile::TempDir::new().unwrap();
        std::fs::write(project.path().join("wj.toml"), "[package]\nname = \"p\"\nversion = \"0.1.0\"\nedition = \"2025\"\n").unwrap();
        std::fs::create_dir_all(project.path().join("src")).unwrap();
        let local = project.path().join("src").join(file.file_name().unwrap());
        std::fs::copy(&file, &local).unwrap();
        let out = project.path().join("out");
        let cold = build(project.path(), &local, &out);
        let warm1 = build(project.path(), &local, &out);
        let warm2 = build(project.path(), &local, &out);
        if cold != warm1 || warm1 != warm2 {
            diverged.push(file.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    assert!(diverged.is_empty(), "cold/warm builds differ for: {diverged:?}");
}
