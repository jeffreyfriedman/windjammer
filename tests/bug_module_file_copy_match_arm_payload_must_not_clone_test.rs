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

//! Copy match-arm payloads (`i32`/`f32`/`bool`) must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum Cell {
    Int(i32),
    Float(f32),
    Flag(bool),
    Empty,
}

pub fn get_int(c: Cell) -> Option<i32> {
    match c {
        Cell::Int(value) => Some(value),
        _ => None,
    }
}

pub fn get_float(c: Cell) -> Option<f32> {
    match c {
        Cell::Float(value) => Some(value),
        _ => None,
    }
}

pub fn get_flag(c: Cell) -> Option<bool> {
    match c {
        Cell::Flag(value) => Some(value),
        _ => None,
    }
}

pub enum StateDataValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    String(string),
}

pub struct State {
    pub data: Map<string, StateDataValue>,
}

impl State {
    pub fn get_data_int(self, key: string) -> Option<i32> {
        match self.data.get(key) {
            Some(StateDataValue::Int(value)) => Some(value),
            _ => None,
        }
    }
    pub fn get_data_float(self, key: string) -> Option<f32> {
        match self.data.get(key) {
            Some(StateDataValue::Float(value)) => Some(value),
            _ => None,
        }
    }
    pub fn get_data_bool(self, key: string) -> Option<bool> {
        match self.data.get(key) {
            Some(StateDataValue::Bool(value)) => Some(value),
            _ => None,
        }
    }
}
"#;

#[test]
fn module_file_copy_match_arm_payload_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("value.clone()");
    assert!(
        !bad,
        "RED Copy match-arm payload cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
