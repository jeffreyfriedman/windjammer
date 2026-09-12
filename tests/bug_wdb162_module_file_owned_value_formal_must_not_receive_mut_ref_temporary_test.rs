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

//! WDB-162: full relational `--module-file` must not pass `&mut Value::Int64(...)`
//! into owned `Value` formals.
//!
//! Product cold gen (`relational_pg_serve_dispatch_port.rs`):
//!   `relational_mvcc_put_version(..., &mut Value::Int64(42))` → E0308 (~98).
//! Tip-cluster of dispatch+execute+mvcc emits owned `Value::Int64(42)` (GREEN).
//!
//! Gate A: minimal multipass must not invent `&mut` on enum temporaries.
//! Gate B: product `relational_module_file` dispatch must not contain `&mut Value::Int64`.

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

pub fn put_version(id: i64, value: Value) -> i64 {
    match value {
        Value::Int64(v) => id + v,
        Value::Null => id,
    }
}

pub fn seed() -> i64 {
    let mut n = put_version(1, Value::Int64(42))
    n = put_version(n, Value::Int64(7))
    n
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
    let test = wdb162_fixture();
    let map = test
        .compile()
        .expect("WDB-162 multipass compile should succeed (codegen may still be wrong)");
    let store_rs = map.get("store.rs").expect("store.rs");

    eprintln!("WDB-162 minimal store.rs:\n{store_rs}");

    assert!(
        !store_rs.contains("&mut Value::Int64") && !store_rs.contains("&mut value::Value::Int64"),
        "WDB-162: minimal multipass must not invent &mut on Value temporaries."
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
         Tip-cluster of dispatch+mvcc alone is clean — tip must not invent &mut on owned Value temps. \
         Product: relational_mvcc_put_version(..., &mut Value::Int64(42)).",
        path.display()
    );
}
