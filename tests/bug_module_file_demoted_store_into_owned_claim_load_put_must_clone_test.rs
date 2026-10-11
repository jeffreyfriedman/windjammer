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

//! demoted `&RelationalMvccStore` into owned job_store load/put must clone.
//!
//! Product residual (~10× RelationalMvccStore←&), tip-out claim/heartbeat/release:
//!   `job_store_load_jobs(&store, …)` / `job_store_put_job(&next_store, …)`
//! while formals are owned `RelationalMvccStore` → E0308.
//! greened tip-out **job_store_port**; satellite claim ports still RED.
//! Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod store
pub mod claim
"#;

const STORE: &str = r#"
pub struct MvccStore {
    pub n: int,
}

/// Owns the store (product job_store_load_jobs / put_job).
pub fn load_jobs(store: MvccStore, snapshot: int) -> MvccStore {
    let _ = snapshot
    MvccStore { n: store.n }
}

pub fn put_job(store: MvccStore, row_id: int, snapshot: int) -> MvccStore {
    let _ = row_id + snapshot
    MvccStore { n: store.n + 1 }
}
"#;

const CLAIM: &str = r#"
use crate::store::MvccStore
use crate::store::load_jobs
use crate::store::put_job

/// Multi-use read demotes toward `&MvccStore` (product job_store_claim_next).
pub fn store_ok(store: MvccStore) -> bool {
    store.n >= 0
}

pub fn claim_next(store: MvccStore, snapshot: int) -> MvccStore {
    let _ = store_ok(store)
    // Product: load_jobs(&store, …) while demoted — must clone
    let loaded = load_jobs(store, snapshot)
    put_job(loaded, 1, snapshot)
}
"#;

const CLAIM_OWNED_MOVE: &str = r#"
use crate::store::MvccStore
use crate::store::load_jobs

/// Owned outer formal, single use — move into owned callee (product claim port line 23).
pub fn claim_move_store(store: MvccStore, snapshot: int) -> MvccStore {
    let loaded = load_jobs(store, snapshot)
    loaded
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("store.wj", STORE);
    test.add_file("claim.wj", CLAIM);
    test
}

#[test]
fn product_claim_port_must_move_store_into_owned_load_jobs() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod observability
"#,
    );
    test.add_file(
        "observability/observability_job_store_port.wj",
        r#"
pub struct JobRecord {
    pub id: string,
}

pub fn job_store_load_jobs(store: MvccStore, snapshot: u64) -> (MvccStore, Vec<JobRecord>) {
    (store, Vec::new())
}

pub struct MvccStore {
    pub n: int,
}
"#,
    );
    test.add_file(
        "observability/observability_job_store_claim_port.wj",
        r#"
use crate::observability::observability_job_store_port::job_store_load_jobs
use crate::observability::observability_job_store_port::MvccStore

pub fn job_store_claim_next(store: MvccStore, snapshot: u64) -> MvccStore {
    let loaded = job_store_load_jobs(store, snapshot)
    loaded.0
}
"#,
    );
    let map = test
        .compile()
        .expect("claim port multipass compile should succeed");
    let claim_rs = map
        .get("observability/observability_job_store_claim_port.rs")
        .expect("claim rs");
    assert!(
        !claim_rs.contains("job_store_load_jobs(&store,"),
        "claim port must not borrow into owned load_jobs. Got:\n{claim_rs}"
    );
}

#[test]
fn module_file_owned_store_into_owned_load_must_move_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("store.wj", STORE);
    test.add_file("claim.wj", CLAIM_OWNED_MOVE);
    let map = test
        .compile()
        .expect("owned-move multipass compile should succeed");
    let claim_rs = map.get("claim.rs").expect("claim.rs");
    assert!(
        !claim_rs.contains("load_jobs(&store,"),
        "owned store must move into owned load_jobs, not borrow. Got:\n{claim_rs}"
    );
    assert!(
        claim_rs.contains("load_jobs(store,") || claim_rs.contains("load_jobs(store.clone(),"),
        "expected move or clone into owned load_jobs. Got:\n{claim_rs}"
    );
}

#[test]
fn module_file_demoted_store_into_owned_load_put_must_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let store_rs = map.get("store.rs").expect("store.rs");
    let claim_rs = map.get("claim.rs").expect("claim.rs");

    eprintln!("store.rs:\n{store_rs}\nclaim.rs:\n{claim_rs}");

    let load_owned = {
        let i = store_rs.find("fn load_jobs").unwrap_or(0);
        let sl = &store_rs[i..store_rs.len().min(i + 100)];
        (sl.contains("store: MvccStore") || sl.contains("store:MvccStore"))
            && !(sl.contains("store: &MvccStore") || sl.contains("store:&MvccStore"))
    };
    let caller_demoted = {
        let i = claim_rs.find("fn claim_next").unwrap_or(0);
        let sl = &claim_rs[i..claim_rs.len().min(i + 100)];
        sl.contains("store: &MvccStore") || sl.contains("store:&MvccStore")
    };
    let bad = claim_rs.contains("load_jobs(store,")
        && !claim_rs.contains("load_jobs(store.clone()")
        && caller_demoted;
    let good = claim_rs.contains("load_jobs(store.clone()");

    if load_owned && caller_demoted && bad && !good {
        panic!(
            "demoted &Store into owned load_jobs without clone. \
             Product: job_store_load_jobs(&store). Got:\n{claim_rs}"
        );
    }
    if load_owned && caller_demoted {
        assert!(
            good || !bad,
            "demoted &Store into owned must clone. Got:\n{claim_rs}"
        );
    }
}
