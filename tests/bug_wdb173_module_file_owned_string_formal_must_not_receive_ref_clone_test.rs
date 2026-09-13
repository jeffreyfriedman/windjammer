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

//! WDB-173: owned `String` formal must not receive `&local.clone()` / `&String`.
//!
//! Product census (~11× `String`←`&String`), e.g. observability reaper:
//!   `job_store_claim_next(…, &worker_id.clone(), …)` while `worker_id: String`
//! → E0308 expected `String`, found `&String`.
//!
//! Opposite of WDB-170 (`&str`.clone() stays `&str`). Here tip over-borrows an
//! owned clone. Signature-driven. Prefer tip greens over dogfood.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod claim
pub mod host
"#;

const CLAIM: &str = r#"
pub struct Store {
    pub n: int,
}

pub struct ClaimResult {
    pub worker_id: string,
    pub ok: bool,
}

/// Owned worker_id consumer (product job_store_claim_next stores the id).
pub fn claim_next(store: Store, worker_id: string, tick: int) -> (Store, ClaimResult) {
    (store, ClaimResult { worker_id: worker_id, ok: tick >= 0 })
}

pub fn heartbeat(store: Store, worker_id: string, tick: int) -> bool {
    store.n >= 0 && worker_id.len() > 0 && tick >= 0
}
"#;

const HOST: &str = r#"
use crate::claim::Store
use crate::claim::claim_next
use crate::claim::heartbeat

pub fn reaper_tick(store: Store, worker_id: string, tick: int) -> bool {
    // Product: claim then heartbeat reuses worker_id — must clone owned, never &worker_id.clone()
    let pair = claim_next(store, worker_id, tick)
    let next = pair.0
    heartbeat(next, worker_id, tick)
}
"#;

fn wdb173_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("claim.wj", CLAIM);
    test.add_file("host.wj", HOST);
    test
}

#[test]
fn wdb173_module_file_owned_string_formal_must_not_receive_ref_clone() {
    let test = wdb173_fixture();
    let map = test
        .compile()
        .expect("WDB-173 multipass compile should succeed");
    let claim_rs = map.get("claim.rs").expect("claim.rs");
    let host_rs = map.get("host.rs").expect("host.rs");

    eprintln!("WDB-173 claim.rs:\n{claim_rs}\nhost.rs:\n{host_rs}");

    let owned_formal = {
        let i = claim_rs.find("fn claim_next").unwrap_or(0);
        let sl = &claim_rs[i..claim_rs.len().min(i + 200)];
        sl.contains("worker_id: String") || sl.contains("worker_id:String")
    };
    let bad_ref_clone = host_rs.contains("&worker_id.clone()")
        || host_rs.contains("claim_next(store, &worker_id")
        || host_rs.contains("claim_next(&store, &worker_id")
        || host_rs.contains("claim_next(store.clone(), &worker_id");
    let good = host_rs.contains("worker_id.clone()")
        && !host_rs.contains("&worker_id.clone()");

    if owned_formal && bad_ref_clone {
        panic!(
            "WDB-173 RED: owned String formal received &worker_id / &worker_id.clone(). \
             Product: job_store_claim_next(…, &worker_id.clone(), …). Got:\n{host_rs}\n{claim_rs}"
        );
    }

    if owned_formal {
        assert!(
            !bad_ref_clone,
            "WDB-173: owned String must not receive &String. Got:\n{host_rs}"
        );
        assert!(
            good || host_rs.contains("claim_next(store, worker_id")
                || host_rs.contains("claim_next(store.clone(), worker_id"),
            "WDB-173: expected owned worker_id / worker_id.clone() at call site. Got:\n{host_rs}"
        );
    }
}

fn wdb173_obs_gen_root() -> PathBuf {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/obs_tip_out");
    if tip.join("observability_job_reaper_host_port.rs").exists() {
        return tip;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen/observability")
}

#[test]
fn wdb173_product_reaper_must_not_ref_clone_worker_id_into_owned() {
    let root = wdb173_obs_gen_root();
    let claim = root.join("observability_job_store_claim_port.rs");
    let host = root.join("observability_job_reaper_host_port.rs");
    if !claim.exists() || !host.exists() {
        eprintln!("WDB-173: skip product gate — claim/host missing");
        return;
    }
    let claim_text = std::fs::read_to_string(&claim).expect("claim");
    let host_text = std::fs::read_to_string(&host).expect("host");
    let owned = claim_text.contains("worker_id: String");
    let bad = owned
        && (host_text.contains("&worker_id.clone()")
            || host_text.contains("job_store_claim_next(&store, snapshot, &worker_id"));
    eprintln!("WDB-173 product owned_worker_id={} bad_ref={}", owned, bad);
    assert!(
        !bad,
        "WDB-173 RED: product reaper still passes &worker_id.clone() into owned String formal."
    );
}
