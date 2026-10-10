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

//! Regression: module file indexed copy cast must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn visit_child(children: Vec<u64>, i: usize) -> u64 {
    children[i] as u64
}

pub fn sum_cast(children: Vec<u64>) -> u64 {
    let mut total: u64 = 0u64
    let mut i: usize = 0
    while i < children.len() {
        total = total + (children[i] as u64)
        i = i + 1
    }
    total
}

// Shape: index+cast into method call args.
pub struct Node {
    pub children: Vec<u64>,
}

pub struct Graph {
    pub nodes: Map<u64, Node>,
}

impl Graph {
    fn update_node_recursive(self, child: u64, pos: Option<f32>) {
        let _ = child
        let _ = pos
    }

    fn count_descendants(self, node_id: u64) -> u32 {
        0u32
    }

    pub fn visit(self, id: u64) {
        let children_copy = match self.nodes.get(id) {
            Some(n) => n.children,
            None => return,
        }
        let mut c: usize = 0
        while c < children_copy.len() {
            self.update_node_recursive(children_copy[c] as u64, Some(1.0))
            c = c + 1
        }
    }

    pub fn count_all(self, id: u64) -> u32 {
        let children_copy = match self.nodes.get(id) {
            Some(n) => n.children,
            None => return 0u32,
        }
        let mut count: u32 = (children_copy.len() as u32)
        let mut i: usize = 0
        while i < children_copy.len() {
            count = count + self.count_descendants(children_copy[i] as u64)
            i = i + 1
        }
        count
    }
}
"#;

#[test]
fn module_file_indexed_copy_cast_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone() as") || rs.contains("].clone()as");
    assert!(
        !bad,
        "RED: indexed Copy element cloned before cast:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
