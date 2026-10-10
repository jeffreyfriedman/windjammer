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

//! Regression: module file owned value formal must not receive mut ref temporary.
//!
//! Root cause: bare-pass demotion to MutBorrowed when struct-literal store of `value`
//! lives under `while` / `versions[i] = Version { value }` / `.push(Version { value })`
//! — `param_stored_in_struct_literal` previously skipped those shapes.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod value
pub mod store
"#;

const VALUE: &str = r#"
pub enum Value {
    Int64(i64),
    Null,
}
"#;

const STORE: &str = r#"
use crate::value::Value

pub struct Version {
    pub row_id: i64,
    pub xmin: u64,
    pub value: Value,
}

pub struct Store {
    pub versions: Vec<Version>,
}

pub fn put_version(store: Store, row_id: i64, xmin: u64, value: Value) -> Store {
    let mut out = store
    let mut i = 0
    while i < out.versions.len() {
        if out.versions[i].row_id == row_id && out.versions[i].xmin == xmin {
            out.versions[i] = Version {
                row_id: row_id,
                xmin: xmin,
                value: value,
            }
            return out
        }
        i = i + 1
    }
    out.versions.push(Version {
        row_id: row_id,
        xmin: xmin,
        value: value,
    })
    out
}

pub fn seed() -> i64 {
    let mut store = Store { versions: Vec::new() }
    store = put_version(store, 7, 10, Value::Int64(42))
    store = put_version(store, 1, 10, Value::Int64(10))
    store.versions.len() as i64
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("value.wj", VALUE);
    test.add_file("store.wj", STORE);
    test
}

#[test]
fn module_file_owned_value_formal_must_not_receive_mut_ref_temporary() {
    let mut test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let store_rs = map.get("store.rs").expect("store.rs");

    eprintln!("store.rs:\n{store_rs}");

    let bad = store_rs.contains("&mut Value::Int64")
        || store_rs.contains("&mut value::Value::Int64")
        || store_rs.contains(", &mut Value::")
        || store_rs.contains(",&mut Value::");

    assert!(
        !bad,
        "RED: owned Value formal received &mut temporary. Got:\n{store_rs}"
    );

    test.cargo_check().expect(
        "owned Value literal args must cargo-check without &mut temporaries.",
    );
}
