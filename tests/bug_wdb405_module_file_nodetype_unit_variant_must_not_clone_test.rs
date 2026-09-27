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

//! WDB-405: Copy `NodeType::PureFunction` must not emit `.clone()`.
//!
//! Prior unit-variant gates (384/392/397/401–404) missed visual-scripting
//! `NodeType`. Product gen/visual_scripting/graph_test.rs still has
//! `NodeType::PureFunction.clone()` into `Node::new(..., node_type: NodeType, ...)`.
//! WJ source is `Node::new(2, NodeType::PureFunction, "Add")`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum NodeType {
    Event,
    PureFunction,
    Branch,
}

pub struct Node {
    pub id: u32,
    pub node_type: NodeType,
    pub name: string,
}

impl Node {
    pub fn new(id: u32, node_type: NodeType, name: string) -> Node {
        Node {
            id: id,
            node_type: node_type,
            name: name,
        }
    }
}

pub fn add_node() -> Node {
    Node::new(2, NodeType::PureFunction, "Add")
}

pub fn event_node() -> Node {
    Node::new(1, NodeType::Event, "BeginPlay")
}
"#;

#[test]
fn wdb405_module_file_nodetype_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-405 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-405 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("NodeType::PureFunction.clone()")
        || rs.contains("NodeType::Event.clone()")
        || rs.contains("NodeType::Branch.clone()");
    assert!(!bad, "WDB-405 RED: Copy NodeType unit variant cloned:\n{rs}");
    test.cargo_check().expect("WDB-405 cargo-check");
}

fn wdb405_search_roots() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![manifest.clone()];
    let git = manifest.join(".git");
    if git.is_file() {
        if let Ok(text) = std::fs::read_to_string(&git) {
            if let Some(line) = text.lines().find(|l| l.starts_with("gitdir:")) {
                let gitdir = PathBuf::from(line.trim_start_matches("gitdir:").trim());
                // `.git/worktrees/<name>` → repo root is 3 ancestors up
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

fn wdb405_product_graph_test_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for dir in wdb405_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/visual_scripting/graph_test.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/graph_test.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/visual_scripting/graph_test.rs"),
        );
        paths.push(dir.join("gen/visual_scripting/graph_test.rs"));
    }
    paths
}

#[test]
fn wdb405_tip_out_game_core_graph_test_must_not_clone_nodetype() {
    let paths = wdb405_product_graph_test_paths();
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            !line.trim_start().starts_with("//")
                && line.contains("NodeType::")
                && line.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-405: graph_test product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-405 RED: tip/product NodeType::*.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
