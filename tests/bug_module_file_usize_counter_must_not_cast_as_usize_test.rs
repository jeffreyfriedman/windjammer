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
//!   `while (i as usize) < self.chunks.len()` with `i: usize`
//! Prefer bare `i < self.chunks.len()`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Manager {
    pub items: Vec<i32>,
}

pub fn count(m: Manager) -> usize {
    let mut i: usize = 0
    let mut n: usize = 0
    while i < m.items.len() {
        n = n + 1
        i = i + 1
    }
    n
}

pub fn find_in_order(m: Manager, target: i32) -> i32 {
    let mut i = 0
    while i < m.items.len() {
        if m.items[i] == target {
            return i
        }
        i = i + 1
    }
    -1
}
"#;

#[test]
fn module_file_usize_counter_must_not_cast_as_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("i as usize") || rs.contains("(i as usize)");
    assert!(
        !bad,
        "RED: usize counter emitted as usize:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

