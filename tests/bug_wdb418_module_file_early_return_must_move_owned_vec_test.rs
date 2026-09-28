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

//! WDB-418: early-return of an owned `Vec` formal must move, not `positions.clone()`.
//!
//! Product `uv_unwrap_algorithm.rs` / `uv_island_packing.rs`:
//!   `return empty_result(positions.clone())` after `positions.len()` (and later
//!   sibling-path uses of `positions`).
//! WJ is `return empty_result(positions)` — that path is exclusive last-use.
//! Distinct from WDB-415 (if-then move, no early return + later sibling uses),
//! WDB-417 (match-arm `chunk.clone()`), and WDB-407 (`Vec` `new` demote).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn empty_result(positions: Vec<i32>) -> i32 {
    positions.len() as i32
}

pub fn unwrap_planar(positions: Vec<i32>, indices: Vec<i32>) -> i32 {
    if positions.len() == 0 || indices.len() < 3 {
        return empty_result(positions)
    }
    let mut i = 0
    while i < indices.len() {
        let vi = indices[i]
        if vi >= positions.len() as i32 {
            return empty_result(positions)
        }
        let p = positions[vi as usize]
        i = i + 1 + p * 0
    }
    positions.len() as i32
}
"#;

#[test]
fn wdb418_module_file_early_return_must_move_owned_vec() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-418 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-418 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("positions.clone()");
    assert!(
        !cloned,
        "WDB-418 RED: early-return cloned owned Vec formal:\n{rs}"
    );
    test.cargo_check().expect("WDB-418 cargo-check");
}

fn wdb418_search_roots() -> Vec<PathBuf> {
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
fn wdb418_tip_out_game_core_empty_result_must_not_clone_positions() {
    let mut paths = Vec::new();
    for dir in wdb418_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/editor/uv_unwrap_algorithm.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/editor/uv_island_packing.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/editor/uv_unwrap_algorithm.rs"),
        );
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/editor/uv_island_packing.rs"),
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
            !t.starts_with("//") && t.contains("empty_result(positions.clone())")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-418: uv unwrap product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-418 RED: tip/product early-return cloned positions in:\n  {}",
        bad_paths.join("\n  ")
    );
}
