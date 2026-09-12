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

//! WDB-162: owned `Value` formal must not receive `&mut Value::Int64(...)` temporaries.
//!
//! Product cold gen (`relational_pg_serve_dispatch_port.rs`):
//!   `relational_mvcc_put_version(..., &mut Value::Int64(42))` → E0308.
//!
//! Root cause: bare-pass demotion to MutBorrowed when struct-literal store of `value`
//! lives under `while` / `versions[i] = Version { value }` / `.push(Version { value })`
//! — `param_stored_in_struct_literal` previously skipped those shapes.
//!
//! Gate A: multipass fixture matching put_version must not invent `&mut` + cargo-check.
//! Gate B: product dispatch gen must not contain `&mut Value::Int64`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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

fn wdb162_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("value.wj", VALUE);
    test.add_file("store.wj", STORE);
    test
}

#[test]
fn wdb162_module_file_owned_value_formal_must_not_receive_mut_ref_temporary() {
    let mut test = wdb162_fixture();
    let map = test
        .compile()
        .expect("WDB-162 multipass compile should succeed (codegen may still be wrong)");
    let store_rs = map.get("store.rs").expect("store.rs");

    eprintln!("WDB-162 store.rs:\n{store_rs}");

    let bad = store_rs.contains("&mut Value::Int64")
        || store_rs.contains("&mut value::Value::Int64")
        || store_rs.contains(", &mut Value::")
        || store_rs.contains(",&mut Value::");

    assert!(
        !bad,
        "WDB-162 RED: owned Value formal received &mut temporary. Got:\n{store_rs}"
    );

    test.cargo_check().expect(
        "WDB-162: owned Value literal args must cargo-check without &mut temporaries.",
    );
}

#[test]
fn wdb162_product_full_relational_module_file_must_not_emit_mut_ref_value_temp() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(
        "windjammerdb/crates/wdb-layers/gen/relational_module_file/relational_pg_serve_dispatch_port.rs",
    );

    if !path.exists() {
        eprintln!(
            "WDB-162: skip product gate — {} missing (run transpile_relational_module_file.sh)",
            path.display()
        );
        return;
    }

    let text = std::fs::read_to_string(&path).expect("read product dispatch gen");
    let bad = text.matches("&mut Value::Int64").count();
    eprintln!(
        "WDB-162 product dispatch {} mut_value_temp_count={}",
        path.display(),
        bad
    );

    assert!(
        bad == 0,
        "WDB-162 RED: full relational --module-file still emits &mut Value::Int64 ({bad}×) in {}. \
         Product: relational_mvcc_put_version(..., &mut Value::Int64(42)).",
        path.display()
    );
}
