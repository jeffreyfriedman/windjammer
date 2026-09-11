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

//! WDB-152: string literal into owned `string` formal across modules must emit
//! `.to_string()` (or equivalent owned String), not bare `&str`.
//!
//! WindjammerDB CQ-C5: tip-sync of `job_store_release_cap_after_heartbeat`
//! composed `job_store_claim_next(store, snapshot, "worker_hb", …)` while
//! claim formal is `worker_id: String` → E0308. Sibling call to
//! `job_store_release_running(..., "worker_hb")` correctly got `.to_string()`.
//! Product dogfood adds missing `.to_string()`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod worker
pub mod cap
"#;

const WORKER: &str = r#"
pub fn claim_next(worker_id: string, tick: i64) -> string {
    worker_id
}

pub fn heartbeat_tick(worker_id: string, tick: i64) -> i64 {
    tick
}
"#;

const CAP: &str = r#"
use crate::worker::claim_next
use crate::worker::heartbeat_tick

pub fn release_cap() -> i64 {
    let claimed = claim_next("worker_hb", 50)
    let tick = heartbeat_tick("worker_hb", 60)
    tick
}
"#;

fn wdb152_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("worker.wj", WORKER);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn wdb152_module_file_string_lit_into_owned_string_formal_must_to_string() {
    let test = wdb152_fixture();
    let map = test
        .compile()
        .expect("WDB-152 multipass compile should succeed (codegen may still be wrong)");
    let cap_rs = map.get("cap.rs").expect("cap.rs");

    eprintln!("WDB-152 cap.rs:\n{cap_rs}");

    let bare_claim = cap_rs.contains("claim_next(\"worker_hb\",")
        || cap_rs.contains("claim_next(\"worker_hb\" ,");
    let bare_hb = cap_rs.contains("heartbeat_tick(\"worker_hb\",")
        || cap_rs.contains("heartbeat_tick(\"worker_hb\" ,");

    if bare_claim || bare_hb {
        panic!(
            "WDB-152 RED: string lit into owned string formal must emit .to_string(). \
             Product: job_store_release_cap claim/heartbeat after tip-sync."
        );
    }

    test.cargo_check().expect(
        "WDB-152: cross-module string lit → owned string must cargo-check.",
    );
}
