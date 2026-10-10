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

//!
//!   `return self.nodes[index as usize].0.clone()`
//! WJ is `return self.nodes[index as usize].0`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct NodeStore {
    pub nodes: Vec<(i32, i32)>,
}

impl NodeStore {
    pub fn x_at(self, index: i32) -> i32 {
        if index < 0 {
            return 0
        }
        if (index as usize) >= self.nodes.len() {
            return 0
        }
        return self.nodes[index as usize].0
    }

    pub fn y_at(self, index: i32) -> i32 {
        if index < 0 {
            return 0
        }
        if (index as usize) >= self.nodes.len() {
            return 0
        }
        return self.nodes[index as usize].1
    }
}
"#;

#[test]
fn module_file_copy_i32_index_tuple_field_return_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".0.clone()") || rs.contains(".1.clone()");
    assert!(
        !bad,
        "RED: Copy i32 indexed tuple field cloned on return:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

