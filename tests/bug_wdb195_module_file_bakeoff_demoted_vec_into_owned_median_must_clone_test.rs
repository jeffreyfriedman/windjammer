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

//! WDB-195: tip-out bakeoff demoted `&Vec<u64>` into owned median must clone.
//!
//! Product residual (~6× Vec←&Vec), tip-out **and** gen bakeoff:
//!   `wave1_opt_bakeoff_run(samples: &Vec<u64>, …)`
//!   → `opt_quiet_median_and_contended(samples)` owned `Vec<u64>` → E0308.
//! Distinct from WDB-185 (gen tpch lag; tip-out tpch owned). Here tip-out itself
//! demotes bakeoff samples. Signature-driven.

use std::path::PathBuf;

#[test]
fn wdb195_tip_out_bakeoff_must_clone_demoted_samples_into_owned_median() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let bakeoff = if tip.join("wave1_opt_bakeoff_port.rs").exists() {
        tip.join("wave1_opt_bakeoff_port.rs")
    } else {
        gen.join("relational/wave1_opt_bakeoff_port.rs")
    };
    let harness = gen.join("stats/opt_harness_port.rs");
    if !bakeoff.exists() || !harness.exists() {
        eprintln!("WDB-195: skip — bakeoff/harness missing");
        return;
    }
    let bake_text = std::fs::read_to_string(&bakeoff).expect("bakeoff");
    let harness_text = std::fs::read_to_string(&harness).expect("harness");
    let median_owned = harness_text
        .contains("fn opt_quiet_median_and_contended(samples: Vec<u64>")
        && !harness_text.contains("fn opt_quiet_median_and_contended(samples: &Vec<u64>");
    let caller_demoted = bake_text.contains("fn wave1_opt_bakeoff_run(samples: &Vec<u64>")
        || bake_text.contains("fn wave1_opt_bakeoff_run_median(samples: &Vec<u64>");
    let bare = bake_text.contains("opt_quiet_median_and_contended(samples)")
        && !bake_text.contains("opt_quiet_median_and_contended(samples.clone()");
    eprintln!(
        "WDB-195 tip-out median_owned={} demoted={} bare={} path={}",
        median_owned,
        caller_demoted,
        bare,
        bakeoff.display()
    );
    assert!(
        !(median_owned && caller_demoted && bare),
        "WDB-195 RED: tip-out bakeoff passes &Vec into owned median. {}",
        bakeoff.display()
    );
}
