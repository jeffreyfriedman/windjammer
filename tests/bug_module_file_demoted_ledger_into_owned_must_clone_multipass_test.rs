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

//! when forwarding into owned harness formals.
//!
//!   `ledger: &mut Ledger` + `phase_b_within_budget(ledger, …)` owned.
//!

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn multipass_demoted_ledger_into_owned_must_clone() {
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
pub struct Ledger {
    pub peak: int,
}

pub fn phase_b_within_budget(ledger: Ledger, limit: int) -> bool {
    ledger.peak <= limit
}
"#,
    );
    test.add_file(
        "wrap.wj",
        r#"
use crate::harness::{Ledger, phase_b_within_budget}

/// Read-only probe demotes toward `&Ledger` while harness stays owned.
pub fn peak_of(ledger: Ledger) -> int {
    ledger.peak
}

pub fn report_econ_query_cpu_within_budget(ledger: Ledger, limit: int) -> bool {
    let _ = peak_of(ledger)
    phase_b_within_budget(ledger, limit)
}
"#,
    );
    let map = test.compile().expect("multipass");
    let wrap = map.get("wrap.rs").expect("wrap.rs");
    let harness = map.get("harness.rs").expect("harness.rs");
    eprintln!("wrap.rs:\n{wrap}\nharness.rs:\n{harness}");
    let harness_owned = harness.contains("ledger: Ledger")
        && !harness.contains("ledger: &Ledger")
        && !harness.contains("ledger: &mut Ledger");
    let wrap_demoted = {
        let i = wrap.find("fn report_econ_query_cpu_within_budget").unwrap_or(0);
        let sl = &wrap[i..wrap.len().min(i + 140)];
        sl.contains("ledger: &Ledger") || sl.contains("ledger: &mut Ledger")
    };
    if harness_owned && wrap_demoted {
        assert!(
            wrap.contains("ledger.clone()") || wrap.contains("(*ledger).clone()"),
            "RED: demoted &ledger into owned harness without clone. Got:\n{wrap}"
        );
    }
    assert!(
        !wrap.contains("ledger: &mut Ledger"),
        "RED: multipass demotes wrapper to &mut Ledger. Got:\n{wrap}"
    );
}

