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
//!   → `opt_quiet_median_and_contended(samples)` owned formal → E0308.
//! Same for `run_hardware_*` → `run_bakeoff(samples, …)`.
//!

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod quiet
pub mod report
"#;

const QUIET: &str = r#"
pub fn median_and_contended(samples: Vec<int>) -> int {
    samples.len() as int
}
"#;

const REPORT: &str = r#"
use crate::quiet::median_and_contended

pub fn probe_len(samples: Vec<int>) -> int {
    samples.len() as int
}

pub fn query_verdict(query_id: int, samples: Vec<int>) -> int {
    let _ = probe_len(samples)
    let _ = query_id
    median_and_contended(samples)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("quiet.wj", QUIET);
    test.add_file("report.wj", REPORT);
    test
}

#[test]
fn module_file_demoted_vec_into_owned_median_must_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let quiet_rs = map.get("quiet.rs").expect("quiet.rs");
    let report_rs = map.get("report.rs").expect("report.rs");

    eprintln!("quiet.rs:\n{quiet_rs}\nreport.rs:\n{report_rs}");

    let median_owned = {
        let i = quiet_rs.find("fn median_and_contended").unwrap_or(0);
        let sl = &quiet_rs[i..quiet_rs.len().min(i + 120)];
        (sl.contains("samples: Vec") || sl.contains("samples:Vec"))
            && !(sl.contains("samples: &Vec") || sl.contains("samples:&Vec"))
    };
    let caller_demoted = {
        let i = report_rs.find("fn query_verdict").unwrap_or(0);
        let sl = &report_rs[i..report_rs.len().min(i + 140)];
        sl.contains("samples: &Vec") || sl.contains("samples:&Vec")
    };
    let call_ok = report_rs.contains("median_and_contended(samples.clone()")
        || report_rs.contains("median_and_contended((*samples).clone()");
    let call_bad = report_rs.contains("median_and_contended(samples)")
        && !report_rs.contains("median_and_contended(samples.clone()");

    if median_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "RED: demoted &Vec into owned median without clone. \
             Got:\n{report_rs}\n{quiet_rs}"
        );
    }

    if median_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "demoted &Vec into owned must clone. Got:\n{report_rs}"
        );
    }
}

