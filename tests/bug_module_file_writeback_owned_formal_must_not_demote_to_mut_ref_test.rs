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

//! writeback helper `fn put(store: T, …) -> T` must not demote to `&mut T`.
//!
//! covers demotion to shared `&T`. Product cold relational gen (2026-09-11)
//! still emits `relational_mvcc_put_version(store: &mut RelationalMvccStore, …, value: &mut Value)
//! -> RelationalMvccStore` and then fails with E0308 (expected owned / found &mut, and
//! `value` field type mismatches). Distinct tip bug class from .
//!
//! Expected: owned formals + owned call sites for writeback `T → T`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod store
pub mod cap
"#;

const STORE: &str = r#"
pub struct MvccStore {
    pub n: i64
}

pub struct Cell {
    pub v: i64
}

pub fn put_version(store: MvccStore, row_id: i64, value: Cell) -> MvccStore {
    MvccStore {
        n: store.n + row_id + value.v
    }
}
"#;

const CAP: &str = r#"
use crate::store::MvccStore
use crate::store::Cell
use crate::store::put_version

pub fn cap_put() -> MvccStore {
    let mut store = MvccStore { n: 0 }
    let cell = Cell { v: 7 }
    store = put_version(store, 1, cell)
    store
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("store.wj", STORE);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn module_file_writeback_owned_formal_must_not_demote_to_mut_ref() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let store_rs = map.get("store.rs").expect("store.rs");
    let cap_rs = map.get("cap.rs").expect("cap.rs");

    let put_mut = store_rs.contains("store: &mut MvccStore")
        || store_rs.contains("value: &mut Cell")
        || store_rs.contains("store:&mut MvccStore");
    let bad_call = cap_rs.contains("put_version(&mut store")
        || cap_rs.contains("put_version(&mut ");

    if put_mut || bad_call {
        eprintln!("store.rs:\n{store_rs}\ncap.rs:\n{cap_rs}");
    }

    assert!(
        !put_mut,
        "put_version(store: MvccStore, …, value: Cell) -> MvccStore must stay owned (not &mut). Product: relational_mvcc_put_version demotes store/value to &mut."
    );
    assert!(
        !bad_call,
        "call sites must pass owned store/value, not &mut."
    );
}
