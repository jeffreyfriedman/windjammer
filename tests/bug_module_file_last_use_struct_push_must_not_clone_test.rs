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

//! a last-use local struct moved into `Vec::push` must not `.clone()`.
//!
//! Product `editor/csg.rs` `bsp_build_into`:
//!   `tree.nodes.push(empty.clone())` and `tree.nodes.push(empty.clone().clone())`
//! WJ is `tree.nodes.push(empty)` and `empty` is not used again.
//! Distinct from (a reused `Vec` argument `.clone().clone()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
fn module_file_last_use_struct_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("empty.clone()"),
        "last-use struct cloned into push:\n{rs}"
    );
    let _ = test.cargo_check();
}
