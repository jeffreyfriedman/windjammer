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
//!   `self.entries[i].key.clone() == key` (blackboard / localization)
//! Prefer `self.entries[i].key == key` (PartialEq for String/&str).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Entry {
    pub key: string,
    pub value: i32,
}

pub struct Board {
    pub entries: Vec<Entry>,
}

pub fn has_key(board: Board, key: string) -> bool {
    let mut i = 0
    while i < board.entries.len() {
        if board.entries[i].key == key {
            return true
        }
        i = i + 1
    }
    false
}
"#;

#[test]
fn module_file_string_field_eq_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".key.clone()") && rs.contains("==");
    assert!(
        !bad,
        "RED: string field eq cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

