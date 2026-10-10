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

//!
//!   `job_store_claim_next(…, &worker_id.clone(), …)` while `worker_id: String`
//! → E0308 expected `String`, found `&String`.
//!

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
    let pair = claim_next(store, worker_id, tick)
    let next = pair.0
    heartbeat(next, worker_id, tick)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("claim.wj", CLAIM);
    test.add_file("host.wj", HOST);
    test
}

#[test]
fn module_file_owned_string_formal_must_not_receive_ref_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let claim_rs = map.get("claim.rs").expect("claim.rs");
    let host_rs = map.get("host.rs").expect("host.rs");

    eprintln!("claim.rs:\n{claim_rs}\nhost.rs:\n{host_rs}");

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
            "RED: owned String formal received &worker_id / &worker_id.clone(). \
             Got:\n{host_rs}\n{claim_rs}"
        );
    }

    if owned_formal {
        assert!(
            !bad_ref_clone,
            "owned String must not receive &String. Got:\n{host_rs}"
        );
        assert!(
            good || host_rs.contains("claim_next(store, worker_id")
                || host_rs.contains("claim_next(store.clone(), worker_id"),
            "expected owned worker_id / worker_id.clone() at call site. Got:\n{host_rs}"
        );
    }
}

