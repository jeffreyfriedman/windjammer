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

//! WDB-213: tpch econ wrappers must not demote `ledger` to `&mut OptEconLedger`
//! when forwarding into owned harness formals.
//!
//! Product residual (~8×), gen tpch (sysbench greened via tip sync):
//!   `ledger: &mut OptEconLedger` + `opt_econ_phase_b_within_budget(ledger, …)` owned.
//! Tip tpch demotes to `&OptEconLedger` (still needs clone). Signature-driven.
//!
//! Hard-fail tip-out + primary `gen/relational/` only. Stale
//! `gen/relational_module_file/` lag is dogfood sync (not tip RED).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

#[test]
fn wdb213_multipass_demoted_ledger_into_owned_must_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod harness
pub mod wrap
"#,
    );
    test.add_file(
        "harness.wj",
        r#"
pub struct OptEconLedger {
    pub peak: int,
}

pub fn opt_econ_phase_b_within_budget(ledger: OptEconLedger, limit: int) -> bool {
    ledger.peak <= limit
}
"#,
    );
    test.add_file(
        "wrap.wj",
        r#"
use crate::harness::{OptEconLedger, opt_econ_phase_b_within_budget}

/// Read-only probe demotes toward `&OptEconLedger` while harness stays owned.
pub fn peak_of(ledger: OptEconLedger) -> int {
    ledger.peak
}

pub fn tpch_econ_query_cpu_within_budget(ledger: OptEconLedger, limit: int) -> bool {
    let _ = peak_of(ledger)
    opt_econ_phase_b_within_budget(ledger, limit)
}
"#,
    );
    let map = test.compile().expect("WDB-213 multipass");
    let wrap = map.get("wrap.rs").expect("wrap.rs");
    let harness = map.get("harness.rs").expect("harness.rs");
    eprintln!("WDB-213 wrap.rs:\n{wrap}\nharness.rs:\n{harness}");
    let harness_owned = harness.contains("ledger: OptEconLedger")
        && !harness.contains("ledger: &OptEconLedger")
        && !harness.contains("ledger: &mut OptEconLedger");
    let wrap_demoted = {
        let i = wrap.find("fn tpch_econ_query_cpu_within_budget").unwrap_or(0);
        let sl = &wrap[i..wrap.len().min(i + 140)];
        sl.contains("ledger: &OptEconLedger") || sl.contains("ledger: &mut OptEconLedger")
    };
    if harness_owned && wrap_demoted {
        assert!(
            wrap.contains("ledger.clone()") || wrap.contains("(*ledger).clone()"),
            "WDB-213 RED: demoted &ledger into owned harness without clone. Got:\n{wrap}"
        );
    }
    assert!(
        !wrap.contains("ledger: &mut OptEconLedger"),
        "WDB-213 RED: tip multipass demotes wrapper to &mut OptEconLedger. Got:\n{wrap}"
    );
}

#[test]
fn wdb213_product_tpch_must_not_forward_mut_ref_ledger_into_owned_econ() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("tpch_opt_port.rs"),
        gen.join("relational/tpch_opt_port.rs"),
        gen.join("relational_module_file/tpch_opt_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("tpch");
        let mut_formal = text.contains("tpch_econ_query_cpu_within_budget(ledger: &mut OptEconLedger")
            || text.contains("tpch_econ_peak_rss_recorded(ledger: &mut OptEconLedger");
        let demoted_ref = text.contains("tpch_econ_query_cpu_within_budget(ledger: &OptEconLedger")
            || text.contains("tpch_econ_peak_rss_recorded(ledger: &OptEconLedger");
        let bare_forward = text.contains("opt_econ_phase_b_within_budget(ledger,")
            || text.contains("opt_econ_peak_rss_recorded(ledger)");
        let clones = text.contains("ledger.clone()") || text.contains("(*ledger).clone()");
        let bad = (mut_formal || demoted_ref) && bare_forward && !clones;
        let path_s = path.to_string_lossy();
        eprintln!(
            "WDB-213 mut={} demoted={} bare={} clones={} bad={} path={}",
            mut_formal,
            demoted_ref,
            bare_forward,
            clones,
            bad,
            path.display()
        );
        if path_s.contains("rel_tip_out") || path_s.contains("/gen/relational/") {
            assert!(
                !bad,
                "WDB-213 RED: tip/product forwards &/&mut OptEconLedger into owned harness without clone. {}",
                path.display()
            );
        } else if bad {
            eprintln!(
                "WDB-213: stale module_file lag (sync tip emit): {}",
                path.display()
            );
        }
    }
    assert!(saw, "WDB-213: tpch_opt missing");
}
