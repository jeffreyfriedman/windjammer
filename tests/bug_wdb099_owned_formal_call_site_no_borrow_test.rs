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

//! WDB-099 / WindjammerDB: owned formals must not get `&arg` at call sites (IR-driven).
//!
//! Multipass regen must not emit:
//!   `opt_run_cost_report(&ledger)` where formal is owned non-Copy `OptEconLedger`
//!   `opt_quiet_median_and_contended(&samples)` where formal is owned `Vec<u64>`
//! → E0308. Call sites must move owned args when the callee formal is Owned.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn wdb099_sources() -> (&'static str, &'static str, &'static str) {
    (
        r#"
pub mod harness
pub mod consumer
"#,
        r#"
/// Cap stand-in for OptEconLedger (non-Copy via string field — matches real ledger shape).
pub struct OptEconLedger {
    pub phase_a_nanos: u64,
    pub phase_b_nanos: u64,
    pub peak_rss_bytes: u64,
    pub label: string,
}

pub fn opt_econ_ledger(a: u64, b: u64, rss: u64) -> OptEconLedger {
    OptEconLedger {
        phase_a_nanos: a,
        phase_b_nanos: b,
        peak_rss_bytes: rss,
        label: "cap",
    }
}

pub fn opt_run_cost_report(ledger: OptEconLedger) -> u64 {
    ledger.phase_a_nanos + ledger.phase_b_nanos + ledger.peak_rss_bytes
}

pub fn opt_quiet_median_and_contended(samples: Vec<u64>) -> u64 {
    if samples.len() == 0 {
        return 0
    }
    samples[0]
}
"#,
        r#"
use crate::harness::OptEconLedger
use crate::harness::opt_econ_ledger
use crate::harness::opt_quiet_median_and_contended
use crate::harness::opt_run_cost_report

pub fn format_econ_markdown(ledger: OptEconLedger) -> u64 {
    opt_run_cost_report(ledger)
}

pub fn workload_verdict(samples: Vec<u64>) -> u64 {
    opt_quiet_median_and_contended(samples)
}

pub fn rebuild_then_report() -> u64 {
    let ledger = opt_econ_ledger(1, 2, 3)
    format_econ_markdown(ledger)
}
"#,
    )
}

fn assert_consumer_matches_harness_ownership(harness: &str, consumer: &str) {
    // IR demotes read-only formals to `&T` / `&Vec<T>` and call sites must match.
    let report_owned = harness.contains("opt_run_cost_report(ledger: OptEconLedger)");
    let report_borrowed = harness.contains("opt_run_cost_report(ledger: &OptEconLedger)");
    assert!(
        report_owned || report_borrowed,
        "expected opt_run_cost_report formal. harness=\n{harness}"
    );
    if report_owned {
        assert!(
            !consumer.contains("opt_run_cost_report(&ledger)"),
            "WDB-099: owned OptEconLedger formal must not receive &ledger. Got:\n{consumer}"
        );
        assert!(
            consumer.contains("opt_run_cost_report(ledger)"),
            "expected move of ledger into owned formal. Got:\n{consumer}"
        );
    } else {
        assert!(
            consumer.contains("opt_run_cost_report(&ledger)"),
            "demoted &OptEconLedger formal requires borrow at call site. Got:\n{consumer}"
        );
    }

    let samples_owned = harness.contains("opt_quiet_median_and_contended(samples: Vec<u64>)");
    let samples_borrowed = harness.contains("opt_quiet_median_and_contended(samples: &Vec<u64>)");
    assert!(
        samples_owned || samples_borrowed,
        "expected opt_quiet_median_and_contended formal. harness=\n{harness}"
    );
    if samples_owned {
        assert!(
            !consumer.contains("opt_quiet_median_and_contended(&samples)"),
            "WDB-099: owned Vec formal must not receive &samples. Got:\n{consumer}"
        );
        assert!(
            consumer.contains("opt_quiet_median_and_contended(samples)"),
            "expected move of samples into owned Vec formal. Got:\n{consumer}"
        );
    } else {
        // Caller may already be `&Vec` (`samples: &Vec<u64>` → pass `samples`) or own then borrow.
        assert!(
            consumer.contains("opt_quiet_median_and_contended(samples)")
                || consumer.contains("opt_quiet_median_and_contended(&samples)"),
            "demoted &Vec formal must be passed by shared ref. Got:\n{consumer}"
        );
        assert!(
            !consumer.contains("opt_quiet_median_and_contended(samples.clone())"),
            "must not clone into &Vec formal. Got:\n{consumer}"
        );
    }
}

fn wdb099_struct_field_sources() -> (&'static str, &'static str, &'static str) {
    (
        r#"
pub mod claims
pub mod suite
"#,
        r#"
pub struct Wave1OptLiveClaims {
    pub claim_ready: bool,
}

pub fn wave1_opt_live_claims_cap(ready: bool) -> Wave1OptLiveClaims {
    Wave1OptLiveClaims { claim_ready: ready }
}
"#,
        r#"
use crate::claims::Wave1OptLiveClaims
use crate::claims::wave1_opt_live_claims_cap

pub fn wave1_opt_live_row_publishable(claim_ready: bool) -> bool {
    claim_ready
}

pub fn wave1_opt_live_suite_verdict(claims: Wave1OptLiveClaims) -> bool {
    wave1_opt_live_row_publishable(claims.claim_ready)
}

pub fn wave1_opt_live_suite_test() -> bool {
    let claims = wave1_opt_live_claims_cap(true)
    wave1_opt_live_suite_verdict(claims)
}
"#,
    )
}

fn assert_suite_no_overborrow(suite: &str) {
    assert!(
        !suite.contains("wave1_opt_live_suite_verdict(&claims)"),
        "WDB-099 Gate C: owned Wave1OptLiveClaims formal must not receive &claims. Got:\n{suite}"
    );
    assert!(
        suite.contains("wave1_opt_live_suite_verdict(claims)"),
        "expected move of claims into owned suite formal. Got:\n{suite}"
    );
}

#[test]
fn wdb099_owned_struct_and_vec_formals_must_not_borrow_at_call_site() {
    let (mod_wj, harness, consumer_src) = wdb099_sources();
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", mod_wj);
    test.add_file("harness.wj", harness);
    test.add_file("consumer.wj", consumer_src);

    let map = test
        .compile()
        .expect("WDB-099 multipass compile should succeed");
    let harness = map
        .get("harness.rs")
        .expect("harness.rs must be generated");
    let consumer = map
        .get("consumer.rs")
        .expect("consumer.rs must be generated");
    assert_consumer_matches_harness_ownership(harness, consumer);
}

/// Owned aggregate struct at suite call site (Wave1OptLiveClaims pattern).
#[test]
fn wdb099_owned_claims_struct_must_not_borrow_at_call_site() {
    let (mod_wj, claims, suite_src) = wdb099_struct_field_sources();
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", mod_wj);
    test.add_file("claims.wj", claims);
    test.add_file("suite.wj", suite_src);

    let map = test
        .compile()
        .expect("WDB-099 Gate C multipass compile should succeed");
    let suite = map.get("suite.rs").expect("suite.rs must be generated");
    assert_suite_no_overborrow(suite);
}
