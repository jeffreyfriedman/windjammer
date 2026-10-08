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

//! WDB-466: indexed Copy tuple **field** into `Vec::push` must not `.clone()`.
//!
//! Product `scene_graph/scene_graph_state.rs` `sort_by_distance`:
//!   `sorted_nodes.push(sorted[k].0.clone())` and `sorted[i].1.clone()`
//! WJ uses bare `sorted[k].0` / `sorted[i].1` (`u64` / `f32`, Copy).
//! Distinct from WDB-439 (tuple field **return**), WDB-465 (clone the whole
//! indexed tuple into push), WDB-363 (clone the element then read a field).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn nearer(sorted: Vec<(u64, f32)>, i: int, j: int) -> bool {
    let a_dist = sorted[i].1
    let b_dist = sorted[j].1
    a_dist > b_dist
}

pub fn ids_in_order(sorted: Vec<(u64, f32)>) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new()
    let mut k: int = 0
    while k < sorted.len() {
        out.push(sorted[k].0)
        k = k + 1
    }
    out
}
"#;

#[test]
fn wdb466_module_file_indexed_copy_tuple_field_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-466 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-466 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".0.clone()") || rs.contains(".1.clone()");
    assert!(
        !bad,
        "WDB-466 RED: indexed Copy tuple field cloned:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb466_search_roots() -> Vec<PathBuf> {
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
fn wdb466_tip_out_game_core_scene_graph_tuple_field_push_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb466_search_roots() {
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
                && (t.contains("sorted[k].0.clone()")
                    || t.contains("sorted[i].1.clone()")
                    || t.contains("sorted[j].1.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-466: scene_graph_state product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-466 RED: tip/product indexed Copy tuple field clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
