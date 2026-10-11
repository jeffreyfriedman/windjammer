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

//! Copy `i32` formal into tuple literal must not `.clone()` when
//! the same formal is also passed to a sibling call in that tuple.
//!
//! Product `ai/astar_grid.rs` `get_neighbors`:
//!   `result.push((x.clone(), y + 1_i32, self.get_cost(x, y + 1_i32)))`
//! WJ is `result.push((x, y + 1, self.get_cost(x, y + 1)))`.
//! Distinct from (formal into **let**), (const into **call**),
//! and (indexed tuple element).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct CostGrid {
    pub base: f32,
}

impl CostGrid {
    pub fn get_cost(self, x: i32, y: i32) -> f32 {
        self.base + (x as f32) * 0.01 + (y as f32) * 0.01
    }

    pub fn get_neighbors(self, x: i32, y: i32) -> Vec<(i32, i32, f32)> {
        let mut result: Vec<(i32, i32, f32)> = Vec::new()
        result.push((x, y + 1, self.get_cost(x, y + 1)))
        result.push((x + 1, y, self.get_cost(x + 1, y)))
        result.push((x, y - 1, self.get_cost(x, y - 1)))
        result.push((x - 1, y, self.get_cost(x - 1, y)))
        result
    }
}
"#;

#[test]
fn module_file_copy_i32_formal_tuple_lit_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("x.clone()") || rs.contains("y.clone()");
    assert!(
        !bad,
        "Copy i32 formal cloned into tuple lit:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
