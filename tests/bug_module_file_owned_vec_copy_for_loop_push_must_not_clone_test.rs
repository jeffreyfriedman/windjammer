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

//! `Vec::push`.
//!
//!   `for child_id in &children_copy { to_remove.push(child_id.clone()) }`
//! WJ uses owned `for child_id in children_copy { to_remove.push(child_id) }`
//! (`child_id: u64`, Copy).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
fn module_file_owned_vec_copy_for_loop_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("child_id.clone()")
        || rs.contains("for child_id in &children_copy");
    assert!(
        !bad,
        "RED: owned Vec<Copy> for-loop demoted/cloned into push:\n{rs}"
    );
    let _ = test.cargo_check();
}

