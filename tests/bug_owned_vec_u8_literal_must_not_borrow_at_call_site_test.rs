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

//! P3.589 / engine leftover: owned `Vec<u8>` formals must take `vec![...]` by
//! value. Product `gen/ecs/component_storage.rs` emits
//! `registry.add(..., &vec![0_u8; 36])` → E0308 (expected Vec, found &Vec).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Registry {}

impl Registry {
    pub fn new() -> Registry { Registry {} }

    pub fn add(self, entity: i64, data: Vec<u8>) {
        let _ = entity
        let _ = data
    }
}

pub fn seed(reg: Registry) {
    reg.add(1, vec![0u8; 36])
    reg.add(2, vec![10u8, 0, 0, 0])
}
"#;

#[test]
fn owned_vec_u8_literal_must_not_borrow_at_call_site() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs").clone();
    eprintln!("P3.589 emit:\n{rs}");
    assert!(
        !rs.contains("&vec!["),
        "P3.589 RED: owned Vec<u8> formal must not borrow vec! literal:\n{rs}"
    );
    assert!(
        rs.contains("vec![") && rs.contains("add("),
        "must still emit add + vec! literal:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
