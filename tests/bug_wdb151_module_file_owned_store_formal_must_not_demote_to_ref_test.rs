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

//! WDB-151: mutating helper that takes owned `T` and returns `T` must not demote
//! the formal to `&T`, and same-module call sites must not pass `&local`.
//!
//! WindjammerDB CQ-C5: tip-sync of `job_store_put_job` /
//! `job_store_load_jobs` demoted `store: RelationalMvccStore` → `&RelationalMvccStore`
//! while `relational_mvcc_put_version` still needs owned, and `job_store_cap_demo`
//! emitted `job_store_put_job(&store, …)` → E0308. Product dogfood restores owned.
//!
//! Expected: owned formal + owned call sites (move/reassign).

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

pub fn put_job(store: MvccStore, row_id: i64) -> MvccStore {
    MvccStore {
        n: store.n + row_id
    }
}

pub fn load_jobs(store: MvccStore) -> (MvccStore, i64) {
    (store, 1)
}
"#;

const CAP: &str = r#"
use crate::store::MvccStore
use crate::store::put_job
use crate::store::load_jobs

pub fn cap_demo() -> MvccStore {
    let mut store = MvccStore { n: 0 }
    store = put_job(store, 1)
    store = put_job(store, 2)
    let loaded = load_jobs(store)
    loaded.0
}
"#;

fn wdb151_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("store.wj", STORE);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn wdb151_module_file_owned_store_formal_must_not_demote_to_ref() {
    let test = wdb151_fixture();
    let map = test
        .compile()
        .expect("WDB-151 multipass compile should succeed (codegen may still be wrong)");
    let store_rs = map.get("store.rs").expect("store.rs");
    let cap_rs = map.get("cap.rs").expect("cap.rs");

    let put_demoted = {
        let i = store_rs.find("fn put_job").unwrap_or(0);
        let sl = &store_rs[i..store_rs.len().min(i + 120)];
        sl.contains("store: &MvccStore")
    };
    let bad_call = cap_rs.contains("put_job(&store") || cap_rs.contains("load_jobs(&store");

    eprintln!("WDB-151 store.rs:\n{store_rs}\ncap.rs:\n{cap_rs}");
    eprintln!("put_demoted={put_demoted} bad_call={bad_call}");

    if put_demoted || bad_call {
        panic!(
            "WDB-151 RED: owned store formal that returns owned must not demote to &T / &local call sites. \
             Product: job_store_put_job / job_store_cap_demo after tip-sync."
        );
    }

    test.cargo_check().expect(
        "WDB-151: owned store put/load chain must cargo-check without & demotion.",
    );
}
