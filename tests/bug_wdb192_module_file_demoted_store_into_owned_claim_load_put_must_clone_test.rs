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

//! WDB-192: demoted `&RelationalMvccStore` into owned job_store load/put must clone.
//!
//! Product residual (~10× RelationalMvccStore←&), tip-out claim/heartbeat/release:
//!   `job_store_load_jobs(&store, …)` / `job_store_put_job(&next_store, …)`
//! while formals are owned `RelationalMvccStore` → E0308.
//! WDB-174 greened tip-out **job_store_port**; satellite claim ports still RED.
//! Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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

fn wdb192_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("store.wj", STORE);
    test.add_file("claim.wj", CLAIM);
    test
}

#[test]
fn wdb192_module_file_demoted_store_into_owned_load_put_must_clone() {
    let test = wdb192_fixture();
    let map = test
        .compile()
        .expect("WDB-192 multipass compile should succeed");
    let store_rs = map.get("store.rs").expect("store.rs");
    let claim_rs = map.get("claim.rs").expect("claim.rs");

    eprintln!("WDB-192 store.rs:\n{store_rs}\nclaim.rs:\n{claim_rs}");

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
            "WDB-192 RED: demoted &Store into owned load_jobs without clone. \
             Product: job_store_load_jobs(&store). Got:\n{claim_rs}"
        );
    }
    if load_owned && caller_demoted {
        assert!(
            good || !bad,
            "WDB-192: demoted &Store into owned must clone. Got:\n{claim_rs}"
        );
    }
}

#[test]
fn wdb192_tip_out_claim_must_clone_demoted_store_into_owned_load_put() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/obs_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let claim = if tip.join("observability_job_store_claim_port.rs").exists() {
        tip.join("observability_job_store_claim_port.rs")
    } else {
        gen.join("observability/observability_job_store_claim_port.rs")
    };
    let store = gen.join("observability/observability_job_store_port.rs");
    if !claim.exists() || !store.exists() {
        eprintln!("WDB-192: skip — claim/store missing");
        return;
    }
    let claim_text = std::fs::read_to_string(&claim).expect("claim");
    let store_text = std::fs::read_to_string(&store).expect("store");
    let load_owned = store_text.contains("fn job_store_load_jobs(store: RelationalMvccStore")
        && !store_text.contains("fn job_store_load_jobs(store: &RelationalMvccStore");
    let bad = claim_text.contains("job_store_load_jobs(&store,")
        || claim_text.contains("job_store_put_job(&next_store,");
    eprintln!(
        "WDB-192 tip-out load_owned={} bad={} path={}",
        load_owned,
        bad,
        claim.display()
    );
    assert!(
        !(load_owned && bad),
        "WDB-192 RED: tip-out claim still passes &Store into owned load/put. {}",
        claim.display()
    );
}
