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
))]

//! WDB-118: `--module-file` emits `let x: Vec<(Key, Value)> = callee(...)` without importing `Key`.
//!
//! WindjammerDB CQ-C5: `document_dremel_nested_to_kv` source has unannotated
//!   `let pairs = document_dremel_to_kv_pairs(...)`
//! but cargo/tip module-file emit annotates `Vec<(Key, Value)>` without `use wdb_types::Key`
//! → rustc E0425. Same class: `Vec<VectorTopKHit>` annotations without importing the hit type.
//!
//! Expected: either omit the annotation, or emit the required `use` for every named type in it.
//! Gate: multipass cargo-check of the fixture (RED until fixed).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod keys
pub mod nest
"#;

const KEYS: &str = r#"
pub struct Key {
    pub id: i64
}

pub struct Value {
    pub n: i64
}

pub fn to_pairs() -> Vec<(Key, Value)> {
    let mut out: Vec<(Key, Value)> = Vec::new()
    out.push((Key { id: 1 }, Value { n: 2 }))
    out
}
"#;

const NEST: &str = r#"
use crate::keys::to_pairs

pub fn nested_len() -> usize {
    let pairs = to_pairs()
    pairs.len()
}
"#;

fn wdb118_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("keys.wj", KEYS);
    test.add_file("nest.wj", NEST);
    test
}

#[test]
fn wdb118_module_file_inferred_let_must_import_annotated_tuple_types() {
    let mut test = wdb118_fixture();
    let map = test
        .compile()
        .expect("WDB-118 multipass compile should succeed (codegen may still be wrong)");
    let nest_rs = map.get("nest.rs").expect("nest.rs must be generated");

    let annotates_key = nest_rs.contains("Vec<(Key, Value)>") || nest_rs.contains("Vec<(keys::Key");
    let imports_key = nest_rs.contains("use crate::keys::Key")
        || nest_rs.contains("use super::keys::Key")
        || nest_rs.lines().any(|l| l.trim().starts_with("use ") && l.contains("Key") && !l.contains("to_pairs"));

    if annotates_key && !imports_key {
        eprintln!("WDB-118 RED emit nest.rs:\n{nest_rs}");
    }

    assert!(
        !annotates_key || imports_key,
        "WDB-118 RED: if codegen annotates Vec<(Key, Value)>, it must import Key/Value. Product: document_dremel_nested_to_kv E0425."
    );

    test.cargo_check().expect(
        "WDB-118 RED: inferred let from Vec<(Key, Value)> callee must cargo-check (import or no annotation).",
    );
}
