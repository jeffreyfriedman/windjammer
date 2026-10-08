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

//! WDB-469: indexed Copy tuple **swap** must not `.clone()`.
//!
//! Product `scene_graph/scene_graph_state.rs` `sort_by_distance`:
//!   `let tmp = sorted[i].clone(); sorted[i] = sorted[j].clone();`
//! WJ uses bare `sorted[i]` / `sorted[j]` (`(u64, f32)` is Copy).
//! Distinct from WDB-465 (whole tuple into `Vec::push`) and WDB-466
//! (tuple **field** `.0` / `.1` into push or compare).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn swap_at(sorted: Vec<(u64, f32)>, i: int, j: int) -> Vec<(u64, f32)> {
    let mut sorted = sorted
    let tmp = sorted[i]
    sorted[i] = sorted[j]
    sorted[j] = tmp
    sorted
}
"#;

#[test]
fn wdb469_module_file_indexed_copy_tuple_swap_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-469 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-469 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("sorted[i].clone()") || rs.contains("sorted[j].clone()");
    assert!(
        !bad,
        "WDB-469 RED: indexed Copy tuple cloned on swap:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb469_search_roots() -> Vec<PathBuf> {
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
fn wdb469_tip_out_game_core_scene_graph_tuple_swap_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb469_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/scene_graph/scene_graph_state.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/scene_graph/scene_graph_state.rs"),
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
            !t.starts_with("//")
                && (t.contains("sorted[i].clone()") || t.contains("sorted[j].clone()"))
                && !t.contains(".0")
                && !t.contains(".1")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-469: scene_graph_state product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-469 RED: tip/product indexed Copy tuple cloned on swap:\n  {}",
        bad_paths.join("\n  ")
    );
}
