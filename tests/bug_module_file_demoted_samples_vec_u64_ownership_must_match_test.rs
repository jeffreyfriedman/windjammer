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

//! demoted `&Vec<u64>` samples — clone into `&Vec` / bare into owned Vec.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod quiet
pub mod claim
"#;

const QUIET: &str = r#"
/// Owns the samples.
pub fn median_and_contended(samples: Vec<int>) -> int {
    samples.len() as int
}
"#;

const CLAIM: &str = r#"
use crate::quiet::median_and_contended

pub fn claim_with_baseline(row_count: int, samples: Vec<int>) -> int {
    row_count + samples.len() as int
}

/// Multi-use demotes caller toward `&Vec`.
pub fn probe(samples: Vec<int>) -> int {
    samples.len() as int
}

pub fn workload_verdict(samples: Vec<int>) -> int {
    let _n = probe(samples)
    median_and_contended(samples)
}

pub fn point_select_claim(row_count: int, samples: Vec<int>) -> int {
    let _n = probe(samples)
    claim_with_baseline(row_count, samples)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("quiet.wj", QUIET);
    test.add_file("claim.wj", CLAIM);
    test
}

#[test]
fn module_file_demoted_samples_vec_ownership_must_match_formals() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let quiet_rs = map.get("quiet.rs").expect("quiet.rs");
    let claim_rs = map.get("claim.rs").expect("claim.rs");

    eprintln!("quiet.rs:\n{quiet_rs}\nclaim.rs:\n{claim_rs}");

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
            "RED(A): demoted &Vec into owned median without clone. \
Got:\n{claim_rs}\n{quiet_rs}"
        );
    }
    if claim_demoted && b_bad && !b_ok {
        panic!(
            "RED(B): demoted &Vec received samples.clone(). \
Got:\n{claim_rs}"
        );
    }

    if median_owned && caller_demoted {
        assert!(
            a_ok || !a_bad,
            "(A): demoted &Vec into owned must clone. Got:\n{claim_rs}"
        );
    }
    if claim_demoted {
        assert!(
            !b_bad || b_ok,
            "(B): demoted &Vec must borrow not clone. Got:\n{claim_rs}"
        );
    }
}
