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

//! owned store formal consumed via `let mut out = store` must stay owned.
//!
//! Product cold gen (`relational_mvcc_*.rs`):
//!   `fn relational_mvcc_put_version(store: &mut RelationalMvccStore, …) -> RelationalMvccStore`
//!   with body `let mut out = store;` → invalid move from `&mut`, plus call sites
//!   `store = relational_mvcc_put_version(&mut store, …)`.
//!
//! Root cause: bare-pass MutBorrowed demotion when consume-via-let-rebind lives under
//! assignment call sites (`store = put_version(store, …)`), which were invisible to
//! the call-hint walk, and no skip/restore for `let mut out = store`.
//!
//! Note: covers early-return `(state, …)` yielding `&mut State`; this gate is
//! the put_version / writeback store formal demotion sibling.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod store
"#;

const STORE: &str = r#"
pub struct Store {
    pub n: i64,
}

pub fn put_version(store: Store, delta: i64) -> Store {
    let mut out = store
    out.n = out.n + delta
    out
}

pub fn seed() -> i64 {
    let mut store = Store { n: 0 }
    store = put_version(store, 7)
    store = put_version(store, 1)
    store.n
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("store.wj", STORE);
    test
}

#[test]
fn module_file_store_consume_rebind_must_stay_owned() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let store_rs = map.get("store.rs").expect("store.rs");

    eprintln!("store.rs:\n{store_rs}");

    let demoted_formal = store_rs.contains("put_version(store: &mut Store")
        || store_rs.contains("put_version(mut store: &mut Store")
        || store_rs.contains("fn put_version(store: &mut ");
    let mut_ref_call = store_rs.contains("put_version(&mut store")
        || store_rs.contains("put_version(&mut ");

    assert!(
        !demoted_formal && !mut_ref_call,
        "store consume-via-let-mut-rebind demoted to &mut. Got:\n{store_rs}"
    );
    assert!(
        store_rs.contains("put_version(mut store: Store")
            || store_rs.contains("put_version(store: Store"),
        "expected owned Store formal. Got:\n{store_rs}"
    );

    test.cargo_check().expect(
        "owned store consume-rebind must cargo-check without &mut formals/calls.",
    );
}
