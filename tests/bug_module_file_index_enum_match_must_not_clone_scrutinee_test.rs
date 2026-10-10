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

//! WDB-372: indexed enum field match must not clone scrutinee (`].value.clone()`).
//!
//! Tip output for a game project `behavior_tree/blackboard.rs`:
//!   `match self.entries[idx as usize].value.clone() { BlackboardValue::Float(v) => … }`
//! Prefer `match &self.entries[idx].value` / `match self.entries[idx].value`.
//! Twin of WDB-351 (match clone scrutinee) + WDB-359 (index field).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum Val {
    Float(f32),
    Bool(bool),
    Text(string),
}

pub struct Entry {
    pub key: string,
    pub value: Val,
}

pub struct Board {
    pub entries: Vec<Entry>,
}

impl Board {
    pub fn get_f32(self, idx: usize) -> Option<f32> {
        match self.entries[idx].value {
            Val::Float(v) => Some(v),
            _ => None,
        }
    }
}
"#;

#[test]
fn module_file_index_enum_match_must_not_clone_scrutinee() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".value.clone()");
    assert!(
        !bad,
        "indexed enum match cloned scrutinee:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
