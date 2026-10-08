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

//! WDB-465: indexed Copy **tuple** into `Vec::push` must not `.clone()`.
//!
//! Product `ai/astar_grid.rs` path reverse:
//!   `rev.push(path[k as usize].clone())`
//! WJ uses bare `rev.push(path[(k as usize)])` (`(i32, i32)` is Copy).
//! Distinct from WDB-423 (indexed tuple **destructure**), WDB-438 (tuple
//! **lit** push), WDB-464 (indexed Copy **u8** push).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn reverse_path(path: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let mut rev: Vec<(i32, i32)> = Vec::new()
    let mut k: int = path.len() - 1
    while k >= 0 {
        rev.push(path[k])
        k = k - 1
    }
    rev
}
"#;

#[test]
fn wdb465_module_file_indexed_copy_tuple_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-465 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-465 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-465 RED: indexed Copy tuple cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb465_search_roots() -> Vec<PathBuf> {
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
fn wdb465_tip_out_game_core_astar_indexed_tuple_push_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb465_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/ai/astar_grid.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/ai/astar_grid.rs"));
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
            !t.starts_with("//") && t.contains("rev.push(") && t.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-465: astar_grid product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-465 RED: tip/product indexed Copy tuple into Vec::push clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
