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

//! WDB-185: product **gen** demoted `&Vec<u64>` into owned median must clone.
//!
//! Census residual (~35× Vec←&Vec). Tip-out `tpch_opt_port` keeps owned
//! `samples: Vec<u64>` (WDB-179 tip-GREEN), but **gen** still demotes:
//!   `tpch_opt_query_verdict(…, samples: &Vec<u64>)`
//!   → `opt_quiet_median_and_contended(samples)` owned formal → E0308.
//! Same for `wave1_opt_hardware_*` → `wave1_opt_bakeoff_run(wdb_samples, …)`.
//!
//! Distinct from WDB-179 product gate (prefers tip-out sysbench). Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod quiet
pub mod tpch
"#;

const QUIET: &str = r#"
pub fn median_and_contended(samples: Vec<int>) -> int {
    samples.len() as int
}
"#;

const TPCH: &str = r#"
use crate::quiet::median_and_contended

/// Multi-use read-only probe demotes toward `&Vec` (product gen tpch_opt_query_verdict).
pub fn probe_len(samples: Vec<int>) -> int {
    samples.len() as int
}

pub fn query_verdict(query_id: int, samples: Vec<int>) -> int {
    let _ = probe_len(samples)
    let _ = query_id
    // Product gen: median_and_contended(samples) while demoted — must clone.
    median_and_contended(samples)
}
"#;

fn wdb185_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("quiet.wj", QUIET);
    test.add_file("tpch.wj", TPCH);
    test
}

#[test]
fn wdb185_module_file_demoted_vec_into_owned_median_must_clone() {
    let test = wdb185_fixture();
    let map = test
        .compile()
        .expect("WDB-185 multipass compile should succeed");
    let quiet_rs = map.get("quiet.rs").expect("quiet.rs");
    let tpch_rs = map.get("tpch.rs").expect("tpch.rs");

    eprintln!("WDB-185 quiet.rs:\n{quiet_rs}\ntpch.rs:\n{tpch_rs}");

    let median_owned = {
        let i = quiet_rs.find("fn median_and_contended").unwrap_or(0);
        let sl = &quiet_rs[i..quiet_rs.len().min(i + 120)];
        (sl.contains("samples: Vec") || sl.contains("samples:Vec"))
            && !(sl.contains("samples: &Vec") || sl.contains("samples:&Vec"))
    };
    let caller_demoted = {
        let i = tpch_rs.find("fn query_verdict").unwrap_or(0);
        let sl = &tpch_rs[i..tpch_rs.len().min(i + 140)];
        sl.contains("samples: &Vec") || sl.contains("samples:&Vec")
    };
    let call_ok = tpch_rs.contains("median_and_contended(samples.clone()")
        || tpch_rs.contains("median_and_contended((*samples).clone()");
    let call_bad = tpch_rs.contains("median_and_contended(samples)")
        && !tpch_rs.contains("median_and_contended(samples.clone()");

    if median_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "WDB-185 RED: demoted &Vec into owned median without clone. \
             Product gen: tpch_opt_query_verdict. Got:\n{tpch_rs}\n{quiet_rs}"
        );
    }

    if median_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "WDB-185: demoted &Vec into owned must clone. Got:\n{tpch_rs}"
        );
    }
}

#[test]
fn wdb185_product_gen_tpch_must_clone_demoted_samples_into_owned_median() {
    // Product residual is in gitignored gen/ (tip-out may already keep owned Vec).
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let tpch = gen.join("relational/tpch_opt_port.rs");
    let harness = gen.join("stats/opt_harness_port.rs");
    if !tpch.exists() || !harness.exists() {
        eprintln!("WDB-185: skip product gen — tpch/harness missing");
        return;
    }
    let tpch_text = std::fs::read_to_string(&tpch).expect("tpch");
    let harness_text = std::fs::read_to_string(&harness).expect("harness");
    let median_owned = harness_text
        .contains("fn opt_quiet_median_and_contended(samples: Vec<u64>")
        && !harness_text.contains("fn opt_quiet_median_and_contended(samples: &Vec<u64>");
    let caller_demoted = tpch_text.contains("fn tpch_opt_query_verdict(query_id: u32, samples: &Vec<u64>");
    let bare = tpch_text.contains("opt_quiet_median_and_contended(samples)")
        && !tpch_text.contains("opt_quiet_median_and_contended(samples.clone()");
    eprintln!(
        "WDB-185 product gen median_owned={} demoted={} bare={} path={}",
        median_owned,
        caller_demoted,
        bare,
        tpch.display()
    );
    assert!(
        !(median_owned && caller_demoted && bare),
        "WDB-185 RED: product gen tpch passes &Vec into owned median. {}",
        tpch.display()
    );
}
