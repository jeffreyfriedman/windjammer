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

//! Recursive buf: Vec<f32> that demotes to &mut Vec must not get buf.clone().
//!
//! WJ source: for i in 0..node.params.len() and self.emit_instruction(buf, ...).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Node {
    pub params: Vec<f32>,
    pub node_type: i32,
}

pub struct CsgScene {
    pub nodes: Vec<Node>,
}

impl CsgScene {
    pub fn len(self) -> i32 {
        self.nodes.len() as i32
    }

    pub fn get_node(self, node_id: i32) -> Option<Node> {
        if node_id < 0 {
            return None
        }
        Some(self.nodes[0])
    }

    pub fn emit_instruction(self, buf: Vec<f32>, opcode: f32) {
        buf.push(opcode)
    }

    pub fn emit_node_instructions(self, node_id: i32, buf: Vec<f32>) {
        if node_id < 0 {
            return
        }
        let node_opt = self.get_node(node_id)
        let node = match node_opt {
            Some(n) => n,
            None => {
                return
            }
        }
        if node.node_type == 0 {
            for i in 0..node.params.len() {
                buf.push(node.params[i])
            }
            self.emit_instruction(buf, 1.0)
        } else {
            self.emit_node_instructions(node_id, buf)
        }
    }
}
"#;

#[test]
fn module_file_impl_match_len_range_must_not_emit_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad_range = rs.contains("0_i32..node.params.len()");
    let bad_clone = rs.contains("buf.clone()");
    assert!(
        !bad_range && !bad_clone,
        "RED: impl/match len range or buf.clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

