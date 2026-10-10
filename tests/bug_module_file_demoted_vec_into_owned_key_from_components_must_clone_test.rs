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

//! demoted `&Vec` into owned `Key::from_components` must clone/move.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Value {
    pub n: int,
}

pub struct Key {
    pub parts: Vec<Value>,
}

pub fn from_components(components: Vec<Value>) -> Key {
    Key { parts: components }
}

pub fn key_from_parts(parts: Vec<Value>) -> Key {
    from_components(parts)
}
"#;

#[test]
fn module_file_owned_from_components_must_not_receive_ref_parts() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn from_components(components: Vec<")
        || rs.contains("fn from_components(mut components: Vec<");
    assert!(
        owned,
        "expected owned Vec formal for from_components:\n{rs}"
    );
    let bad = rs.contains("from_components(&parts)");
    assert!(
        !bad,
        "RED owned from_components received &parts (must clone/move):\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
