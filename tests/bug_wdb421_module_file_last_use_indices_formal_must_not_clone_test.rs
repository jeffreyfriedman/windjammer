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
    feature = "codegen_tests",
))]

//! WDB-421: last-use of an owned `Vec` formal into a callee must move, not
//! `indices.clone()`.
//!
//! Product `uv_unwrap_algorithm.rs`:
//!   `per_vertex_average_uv(positions.len(), indices.clone(), …)`
//! after earlier `indices[i]` / `indices.len()`.
//! WJ is `per_vertex_average_uv(positions.len(), indices, …)`.
//! Distinct from WDB-418 (early-return of formal into `empty_result`),
//! WDB-420 (return of a local `result.clone()`), and WDB-415 (if-then formal).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn finish(n: i32, indices: Vec<i32>) -> i32 {
    n + indices.len() as i32
}

pub fn unwrap(positions: Vec<i32>, indices: Vec<i32>) -> i32 {
    let mut i = 0
    while i < indices.len() {
        let _v = indices[i]
        i = i + 1
    }
    finish(positions.len() as i32, indices)
}
"#;

#[test]
fn wdb421_module_file_last_use_indices_formal_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-421 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-421 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("indices.clone()");
    assert!(
        !cloned,
        "WDB-421 RED: last-use indices formal cloned into callee:\n{rs}"
    );
    test.cargo_check().expect("WDB-421 cargo-check");
}

fn wdb421_search_roots() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![manifest.clone()];
    let git = manifest.join(".git");
    if git.is_file() {
        if let Ok(text) = std::fs::read_to_string(&git) {
            if let Some(line) = text.lines().find(|l| l.starts_with("gitdir:")) {
                let gitdir = PathBuf::from(line.trim_start_matches("gitdir:").trim());
                if let Some(repo) = gitdir.ancestors().nth(3) {
                    roots.push(repo.to_path_buf());
                    if let Some(src_wj) = repo.parent() {
                        roots.push(src_wj.to_path_buf());
                    }
                }
            }
        }
    }
    let mut walked = manifest;
    for _ in 0..8 {
        roots.push(walked.clone());
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    roots
}

#[test]
fn wdb421_tip_out_game_core_avg_uv_must_not_clone_indices() {
    let mut paths = Vec::new();
    for dir in wdb421_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/editor/uv_unwrap_algorithm.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/editor/uv_unwrap_algorithm.rs"),
        );
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            let t = line.trim_start();
            !t.starts_with("//") && t.contains("indices.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-421: uv_unwrap_algorithm product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-421 RED: tip/product last-use cloned indices in:\n  {}",
        bad_paths.join("\n  ")
    );
}
