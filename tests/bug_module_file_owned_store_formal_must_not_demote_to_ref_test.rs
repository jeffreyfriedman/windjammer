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

//! mutating helper that takes owned `T` and returns `T` must not demote
//! the formal to `&T`, and same-module call sites must not pass `&local`.
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

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("store.wj", STORE);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn module_file_owned_store_formal_must_not_demote_to_ref() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let store_rs = map.get("store.rs").expect("store.rs");
    let cap_rs = map.get("cap.rs").expect("cap.rs");

    let put_demoted = {
        let i = store_rs.find("fn put_job").unwrap_or(0);
        let sl = &store_rs[i..store_rs.len().min(i + 120)];
        sl.contains("store: &MvccStore")
    };
    let bad_call = cap_rs.contains("put_job(&store") || cap_rs.contains("load_jobs(&store");

    eprintln!("store.rs:\n{store_rs}\ncap.rs:\n{cap_rs}");
    eprintln!("put_demoted={put_demoted} bad_call={bad_call}");

    if put_demoted || bad_call {
        panic!(
            "RED owned store formal that returns owned must not demote to &T / &local call sites. \
"
        );
    }

    test.cargo_check().expect(
        "owned store put/load chain must cargo-check without & demotion.",
    );
}
