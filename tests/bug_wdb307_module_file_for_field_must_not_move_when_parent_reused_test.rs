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

//! WDB-307: `for x in owned.field` must not move when the parent is borrowed later.
//!
//! Product tip-out/gen vector_topk:
//!   `for node in index.graph.nodes { vector_topk_score_node(&index, &node, …) }`
//! → E0382 partial move of `index.graph.nodes`. Prefer `for node in &index.graph.nodes`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Node {
    pub id: i64,
}

pub struct Graph {
    pub nodes: Vec<Node>,
}

pub struct Index {
    pub graph: Graph,
}

pub fn score_node(index: Index, node: Node) -> i64 {
    index.graph.nodes.len() as i64 + node.id
}

pub fn topk_from_index(index: Index) -> i64 {
    let mut total = 0
    for node in index.graph.nodes {
        total = total + score_node(index, node)
    }
    total
}
"#;

#[test]
fn wdb307_module_file_for_field_must_not_move_when_parent_reused() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-307 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-307 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("for node in index.graph.nodes")
        && !rs.contains("for node in &index.graph.nodes");
    assert!(
        !bad,
        "WDB-307 RED: MultiFile moves index.graph.nodes while parent reused:\n{rs}"
    );
    test.cargo_check().expect("WDB-307 cargo-check");
}

#[test]
fn wdb307_tip_out_vector_topk_must_borrow_nodes_in_for() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("vector_topk_port.rs"),
        tip.join("vector/vector_topk_port.rs"),
        gen.join("vector/vector_topk_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("topk");
        let bad = text.contains("for node in index.graph.nodes")
            && text.contains("&index")
            && !text.contains("for node in &index.graph.nodes");
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-307: tip-out/gen vector_topk missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-307 RED: tip-out/product moves index.graph.nodes in for-loop in:\n  {}",
        bad_paths.join("\n  ")
    );
}
