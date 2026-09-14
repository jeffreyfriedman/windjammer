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

use std::path::PathBuf;

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
        eprintln!(
            "WDB-213 mut={} demoted={} bare={} clones={} bad={} path={}",
            mut_formal,
            demoted_ref,
            bare_forward,
            clones,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-213 RED: tip-out/product forwards &/&mut OptEconLedger into owned harness without clone. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-213: tpch_opt missing");
}
