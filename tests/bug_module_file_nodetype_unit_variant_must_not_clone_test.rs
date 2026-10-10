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

//! Copy `NodeType::PureFunction` must not emit `.clone()`.
//!
//! Prior unit-variant gates (384/392/397/401–404) missed visual-scripting
//! `NodeType::PureFunction.clone()` into `Node::new(..., node_type: NodeType, ...)`.
//! WJ source is `Node::new(2, NodeType::PureFunction, "Add")`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
fn module_file_nodetype_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("NodeType::PureFunction.clone()")
        || rs.contains("NodeType::Event.clone()")
        || rs.contains("NodeType::Branch.clone()");
    assert!(!bad, "RED Copy NodeType unit variant cloned:\n{rs}");
    test.cargo_check().expect("cargo-check");
}
