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

//! DB-486: a `string` inserted into different maps on successive `if` arms
//! must not `.clone().clone()`. One clone per arm that is not the last use
//! is enough.
//!
//! Product `save/data.rs`:
//!   `data.int_fields.insert(key.clone().clone(), ...)`
//!   `data.float_fields.insert(key.clone().clone(), ...)`
//!   `data.bool_fields.insert(key.clone().clone(), ...)`
//!   `data.string_fields.insert(key.clone(), val.clone())`
//! WJ inserts `key` once per `if section == ...` arm.
//! Distinct from DB-479 (one map: `contains_key`, `insert`, `get_mut`) and
//! DB-485 (one call plus a later `match` arm).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
use std::collections::HashMap

pub fn store(section: string, key: string, val: string) -> int {
    let mut ints: HashMap<string, int> = HashMap::new()
    let mut floats: HashMap<string, f32> = HashMap::new()
    let mut texts: HashMap<string, string> = HashMap::new()
    if section == "int" {
        ints.insert(key, 1)
    }
    if section == "float" {
        floats.insert(key, 1.0)
    }
    if section == "string" {
        texts.insert(key, val)
    }
    ints.len() + floats.len() + texts.len()
}
"#;

#[test]
fn module_file_if_arm_map_key_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "if-arm map key double-cloned into insert:\n{rs}"
    );
    let _ = test.cargo_check();
}
