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

//! WDB-463: owned `Vec<Copy>` for-loop must not demote to `&` + `.clone()` into
//! `Vec::push`.
//!
//! Product `scene_graph/scene_graph_state.rs` `remove_node`:
//!   `for child_id in &children_copy { to_remove.push(child_id.clone()) }`
//! WJ uses owned `for child_id in children_copy { to_remove.push(child_id) }`
//! (`child_id: u64`, Copy).
//! Distinct from WDB-462 (HashMap.keys() inherently borrowed), WDB-457 (owned
//! newtype local), WDB-431 (Copy u64 **field** into push).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Node {
    pub children: Vec<u64>,
}

pub struct SceneGraph {
    pub nodes: Map<u64, Node>,
}

impl SceneGraph {
    pub fn new() -> SceneGraph {
        SceneGraph { nodes: Map::new() }
    }

    pub fn remove_node(self, node_id: u64) {
        let mut to_remove: Vec<u64> = Vec::new()
        to_remove.push(node_id)
        let mut i: int = 0
        while i < to_remove.len() {
            let current = to_remove[i]
            if let Some(node) = self.nodes.get(current) {
                let children_copy = node.children
                for child_id in children_copy {
                    to_remove.push(child_id)
                }
            }
            i = i + 1
        }
    }
}
"#;

#[test]
fn wdb463_module_file_owned_vec_copy_for_loop_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-463 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-463 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("child_id.clone()")
        || rs.contains("for child_id in &children_copy");
    assert!(
        !bad,
        "WDB-463 RED: owned Vec<Copy> for-loop demoted/cloned into push:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb463_search_roots() -> Vec<PathBuf> {
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
fn wdb463_tip_out_game_core_scene_graph_owned_vec_copy_for_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb463_search_roots() {
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
                && (t.contains("to_remove.push(child_id.clone())")
                    || t.contains("for child_id in &children_copy"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-463: scene_graph_state product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-463 RED: tip/product owned Vec<Copy> for-loop demoted/cloned:\n  {}",
        bad_paths.join("\n  ")
    );
}
