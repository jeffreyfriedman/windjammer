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

//! WDB-433: Copy `u64` **field** cloned before cast/arith must not `.clone()`.
//!
//! Product `scene_graph/scene_graph_state.rs`:
//!   `total += node.mesh_id.clone() as u64`
//! WJ is `total = total + node.mesh_id`.
//! Distinct from WDB-431 (`material_id.clone()` into insert), WDB-432
//! (`children_copy[i].clone() as u64` indexed), and WDB-423 (`offsets[i].clone()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Node {
    pub mesh_id: u64,
}

pub fn sum_mesh_ids(nodes: Vec<Node>) -> u64 {
    let mut total: u64 = 0u64
    for node in nodes {
        if node.mesh_id > 0u64 {
            total = total + node.mesh_id
        }
    }
    total
}
"#;

#[test]
fn wdb433_module_file_copy_field_cast_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-433 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-433 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("mesh_id.clone()");
    assert!(
        !bad,
        "WDB-433 RED: Copy field cloned before arith/cast:\n{rs}"
    );
    test.cargo_check().expect("WDB-433 cargo-check");
}

fn wdb433_search_roots() -> Vec<PathBuf> {
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
fn wdb433_tip_out_game_core_mesh_id_cast_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb433_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/scene_graph/scene_graph_state.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/scene_graph/scene_graph_state.rs"));
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
            !t.starts_with("//") && t.contains("mesh_id.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-433: scene_graph_state product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-433 RED: tip/product cloned Copy mesh_id before cast/arith:\n  {}",
        bad_paths.join("\n  ")
    );
}
