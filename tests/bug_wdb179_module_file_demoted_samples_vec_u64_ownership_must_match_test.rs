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

//! WDB-179: demoted `&Vec<u64>` samples — clone into `&Vec` / bare into owned Vec.
//!
//! Product residual after tip-out sync (sysbench/tpch opt):
//!   A) `opt_quiet_median_and_contended(samples)` with `samples: &Vec<u64>` while
//!      formal is owned `Vec<u64>` → Vec←&Vec (WDB-175 class, non-u8).
//!   B) `claim_with_baseline(…, samples.clone(), …)` while formal is `&Vec<u64>`
//!      → &Vec←Vec via `.clone()` (WDB-171 class, non-u8).
//!
//! Signature-driven. Prefer tip greens over dogfood.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod quiet
pub mod claim
"#;

const QUIET: &str = r#"
/// Owns the samples (product opt_quiet_median_and_contended keeps Vec).
pub fn median_and_contended(samples: Vec<int>) -> int {
    samples.len() as int
}
"#;

const CLAIM: &str = r#"
use crate::quiet::median_and_contended

/// Read-only workload — tip demotes to `&Vec<int>` (product claim_with_baseline).
pub fn claim_with_baseline(row_count: int, samples: Vec<int>) -> int {
    row_count + samples.len() as int
}

/// Multi-use demotes caller toward `&Vec`.
pub fn probe(samples: Vec<int>) -> int {
    samples.len() as int
}

pub fn workload_verdict(samples: Vec<int>) -> int {
    let _n = probe(samples)
    // Product A: median_and_contended(samples) while demoted — must clone into owned.
    median_and_contended(samples)
}

pub fn point_select_claim(row_count: int, samples: Vec<int>) -> int {
    let _n = probe(samples)
    // Product B: claim_with_baseline(…, samples.clone()) into demoted &Vec — must borrow.
    claim_with_baseline(row_count, samples)
}
"#;

fn wdb179_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("quiet.wj", QUIET);
    test.add_file("claim.wj", CLAIM);
    test
}

#[test]
fn wdb179_module_file_demoted_samples_vec_ownership_must_match_formals() {
    let test = wdb179_fixture();
    let map = test
        .compile()
        .expect("WDB-179 multipass compile should succeed");
    let quiet_rs = map.get("quiet.rs").expect("quiet.rs");
    let claim_rs = map.get("claim.rs").expect("claim.rs");

    eprintln!("WDB-179 quiet.rs:\n{quiet_rs}\nclaim.rs:\n{claim_rs}");

    let median_owned = {
        let i = quiet_rs.find("fn median_and_contended").unwrap_or(0);
        let sl = &quiet_rs[i..quiet_rs.len().min(i + 120)];
        (sl.contains("samples: Vec<") || sl.contains("samples:Vec<"))
            && !(sl.contains("samples: &Vec") || sl.contains("samples:&Vec"))
    };
    let claim_demoted = {
        let i = claim_rs.find("fn claim_with_baseline").unwrap_or(0);
        let sl = &claim_rs[i..claim_rs.len().min(i + 140)];
        sl.contains("samples: &Vec") || sl.contains("samples:&Vec")
    };
    let caller_demoted = {
        let i = claim_rs.find("fn workload_verdict").unwrap_or(0);
        let sl = &claim_rs[i..claim_rs.len().min(i + 100)];
        sl.contains("samples: &Vec") || sl.contains("samples:&Vec")
    };

    // A: demoted caller → owned median must clone
    let a_ok = claim_rs.contains("median_and_contended(samples.clone()")
        || claim_rs.contains("median_and_contended((*samples).clone()")
        || claim_rs.contains("median_and_contended(samples.to_vec()");
    let a_bad = claim_rs.contains("median_and_contended(samples)")
        && !claim_rs.contains("median_and_contended(samples.clone()");

    // B: demoted claim formal must not receive samples.clone()
    let b_bad = claim_demoted
        && claim_rs.contains("claim_with_baseline(row_count, samples.clone()")
        && !claim_rs.contains("claim_with_baseline(row_count, samples)")
        && !claim_rs.contains("claim_with_baseline(row_count, &samples");
    let b_ok = claim_rs.contains("claim_with_baseline(row_count, samples)")
        || claim_rs.contains("claim_with_baseline(row_count, &samples");

    if median_owned && caller_demoted && a_bad && !a_ok {
        panic!(
            "WDB-179 RED(A): demoted &Vec into owned median without clone. \
             Product: opt_quiet_median_and_contended(samples). Got:\n{claim_rs}\n{quiet_rs}"
        );
    }
    if claim_demoted && b_bad && !b_ok {
        panic!(
            "WDB-179 RED(B): demoted &Vec received samples.clone(). \
             Product: claim_with_baseline(…, samples.clone()). Got:\n{claim_rs}"
        );
    }

    if median_owned && caller_demoted {
        assert!(
            a_ok || !a_bad,
            "WDB-179(A): demoted &Vec into owned must clone. Got:\n{claim_rs}"
        );
    }
    if claim_demoted {
        assert!(
            !b_bad || b_ok,
            "WDB-179(B): demoted &Vec must borrow not clone. Got:\n{claim_rs}"
        );
    }
}

#[test]
fn wdb179_product_sysbench_samples_vec_ownership_must_match() {
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen/relational");
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let sysbench = if tip.join("sysbench_opt_port.rs").exists() {
        tip.join("sysbench_opt_port.rs")
    } else {
        gen.join("sysbench_opt_port.rs")
    };
    if !sysbench.exists() {
        eprintln!("WDB-179: skip product — sysbench missing");
        return;
    }
    let text = std::fs::read_to_string(&sysbench).expect("sysbench");
    let mut bad = Vec::new();
    // A: bare demoted samples into owned median helper
    if text.contains("opt_quiet_median_and_contended(samples)")
        && !text.contains("opt_quiet_median_and_contended(samples.clone()")
        && text.contains("samples: &Vec<u64>")
    {
        bad.push("A: median_and_contended(samples) with demoted &Vec".to_string());
    }
    // B: samples.clone() into demoted &Vec claim formal
    if text.contains("samples.clone()")
        && text.contains("sysbench_point_select_claim_with_baseline(")
        && text.contains("samples: &Vec<u64>")
    {
        // claim_with_baseline formal is &Vec — clone is wrong
        if text.contains("buffer_budget_permille, samples.clone(),")
            || text.contains("buffer_budget_permille(), samples.clone(),")
        {
            bad.push("B: claim_with_baseline(…, samples.clone(), …) into &Vec".to_string());
        }
    }
    eprintln!("WDB-179 product bad={} path={}", bad.len(), sysbench.display());
    assert!(
        bad.is_empty(),
        "WDB-179 RED: sysbench samples Vec ownership mismatches:\n{}",
        bad.join("\n")
    );
}
