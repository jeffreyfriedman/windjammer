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

//! WDB-183: demoted `&OptEconLedger` into owned econ formal must clone.
//!
//! Product residual (~8× OptEconLedger←&OptEconLedger), tip-out sysbench:
//!   `sysbench_econ_query_cpu_within_budget(ledger: &OptEconLedger, …)`
//!   calls `opt_econ_phase_b_within_budget(ledger, …)` with owned formal
//! → E0308. Same class as WDB-178 (dated artifact) for economics ledger.
//! Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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

/// Owned ledger consumer (product opt_econ_phase_b_within_budget).
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
    // Product: phase_b_within_budget(ledger, …) while demoted — must clone.
    phase_b_within_budget(ledger, budget_nanos)
}

pub fn claim_peak_rss_recorded(ledger: EconLedger) -> bool {
    let _ok = ledger_ok(ledger)
    peak_rss_recorded(ledger)
}
"#;

fn wdb183_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("econ.wj", ECON);
    test.add_file("claim.wj", CLAIM);
    test
}

#[test]
fn wdb183_module_file_demoted_econ_ledger_into_owned_must_clone() {
    let test = wdb183_fixture();
    let map = test
        .compile()
        .expect("WDB-183 multipass compile should succeed");
    let econ_rs = map.get("econ.rs").expect("econ.rs");
    let claim_rs = map.get("claim.rs").expect("claim.rs");

    eprintln!("WDB-183 econ.rs:\n{econ_rs}\nclaim.rs:\n{claim_rs}");

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
            "WDB-183 RED: demoted &EconLedger into owned phase_b without clone. \
             Product: opt_econ_phase_b_within_budget(ledger). Got:\n{claim_rs}\n{econ_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "WDB-183: demoted &OptEconLedger into owned must clone. Got:\n{claim_rs}"
        );
    }
}

#[test]
fn wdb183_tip_out_sysbench_must_clone_demoted_ledger_into_owned_econ() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let sysbench = if tip.join("sysbench_opt_port.rs").exists() {
        tip.join("sysbench_opt_port.rs")
    } else {
        gen.join("relational/sysbench_opt_port.rs")
    };
    let econ = gen.join("stats/opt_harness_port.rs");
    if !sysbench.exists() || !econ.exists() {
        eprintln!("WDB-183: skip tip-out — sysbench/econ missing");
        return;
    }
    let sb_text = std::fs::read_to_string(&sysbench).expect("sysbench");
    let econ_text = std::fs::read_to_string(&econ).expect("econ");
    let callee_owned = econ_text.contains("fn opt_econ_phase_b_within_budget(ledger: OptEconLedger")
        && !econ_text.contains("fn opt_econ_phase_b_within_budget(ledger: &OptEconLedger");
    let caller_demoted = sb_text.contains("fn sysbench_econ_query_cpu_within_budget(ledger: &OptEconLedger");
    let bare = sb_text.contains("opt_econ_phase_b_within_budget(ledger,")
        && !sb_text.contains("opt_econ_phase_b_within_budget(ledger.clone()");
    eprintln!(
        "WDB-183 tip-out callee_owned={} demoted={} bare={} path={}",
        callee_owned,
        caller_demoted,
        bare,
        sysbench.display()
    );
    assert!(
        !(callee_owned && caller_demoted && bare),
        "WDB-183 RED: tip-out sysbench passes &OptEconLedger into owned opt_econ. {}",
        sysbench.display()
    );
}
