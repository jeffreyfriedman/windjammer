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

//! WDB-474: a last-use local struct moved into `Vec::push` must not `.clone()`.
//!
//! Product `editor/csg.rs` `bsp_build_into`:
//!   `tree.nodes.push(empty.clone())` and `tree.nodes.push(empty.clone().clone())`
//! WJ is `tree.nodes.push(empty)` and `empty` is not used again.
//! Distinct from WDB-471 (a reused `Vec` argument `.clone().clone()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Node {
    pub front: int,
    pub polys: Vec<int>,
}

pub fn add_empty(nodes: Vec<Node>) -> Vec<Node> {
    let mut nodes = nodes
    let empty = Node { front: -1, polys: Vec::new() }
    nodes.push(empty)
    nodes
}
"#;

#[test]
fn wdb474_module_file_last_use_struct_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-474 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-474 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("empty.clone()"),
        "WDB-474 RED: last-use struct cloned into push:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb474_search_roots() -> Vec<PathBuf> {
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
fn wdb474_tip_out_csg_empty_node_push_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb474_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/editor/csg.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/editor/csg.rs"));
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
            !t.starts_with("//") && t.contains("push(empty.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-474: csg product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-474 RED: tip/product last-use struct cloned into push:\n  {}",
        bad_paths.join("\n  ")
    );
}
