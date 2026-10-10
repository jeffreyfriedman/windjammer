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

//! demoted `&OptEconLedger` into owned econ formal must clone.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod econ
pub mod claim
"#;

const ECON: &str = r#"
pub struct EconLedger {
    pub phase_a_nanos: int,
    pub phase_b_nanos: int,
    pub peak_rss_bytes: int,
}

/// Owned ledger consumer.
pub fn phase_b_within_budget(ledger: EconLedger, budget_nanos: int) -> bool {
    ledger.phase_b_nanos <= budget_nanos && ledger.peak_rss_bytes > 0
}

pub fn peak_rss_recorded(ledger: EconLedger) -> bool {
    ledger.peak_rss_bytes > 0
}
"#;

const CLAIM: &str = r#"
use crate::econ::EconLedger
use crate::econ::phase_b_within_budget
use crate::econ::peak_rss_recorded

/// Multi-use read-only probe demotes toward `&EconLedger`.
pub fn ledger_ok(ledger: EconLedger) -> bool {
    ledger.peak_rss_bytes > 0 && ledger.phase_b_nanos >= 0
}

pub fn claim_query_cpu_within_budget(ledger: EconLedger, budget_nanos: int) -> bool {
    let _ok = ledger_ok(ledger)
    phase_b_within_budget(ledger, budget_nanos)
}

pub fn claim_peak_rss_recorded(ledger: EconLedger) -> bool {
    let _ok = ledger_ok(ledger)
    peak_rss_recorded(ledger)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("econ.wj", ECON);
    test.add_file("claim.wj", CLAIM);
    test
}

#[test]
fn module_file_demoted_econ_ledger_into_owned_must_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let econ_rs = map.get("econ.rs").expect("econ.rs");
    let claim_rs = map.get("claim.rs").expect("claim.rs");

    eprintln!("econ.rs:\n{econ_rs}\nclaim.rs:\n{claim_rs}");

    let callee_owned = {
        let i = econ_rs.find("fn phase_b_within_budget").unwrap_or(0);
        let sl = &econ_rs[i..econ_rs.len().min(i + 160)];
        (sl.contains("ledger: EconLedger") || sl.contains("ledger:EconLedger"))
            && !(sl.contains("ledger: &EconLedger") || sl.contains("ledger:&EconLedger"))
    };
    let caller_demoted = {
        let i = claim_rs
            .find("fn claim_query_cpu_within_budget")
            .unwrap_or(0);
        let sl = &claim_rs[i..claim_rs.len().min(i + 160)];
        sl.contains("ledger: &EconLedger") || sl.contains("ledger:&EconLedger")
    };
    let call_ok = claim_rs.contains("phase_b_within_budget(ledger.clone()")
        || claim_rs.contains("phase_b_within_budget((*ledger).clone()");
    let call_bad = claim_rs.contains("phase_b_within_budget(ledger,")
        && !claim_rs.contains("ledger.clone()");

    if callee_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "RED demoted &EconLedger into owned phase_b without clone. \
Got:\n{claim_rs}\n{econ_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "demoted &OptEconLedger into owned must clone. Got:\n{claim_rs}"
        );
    }
}
