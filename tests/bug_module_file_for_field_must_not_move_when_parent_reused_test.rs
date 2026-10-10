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

//! `for x in owned.field` must not move when the parent is borrowed later.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
fn module_file_for_field_must_not_move_when_parent_reused() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("for node in index.graph.nodes")
        && !rs.contains("for node in &index.graph.nodes");
    assert!(
        !bad,
        "RED MultiFile moves index.graph.nodes while parent reused:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
