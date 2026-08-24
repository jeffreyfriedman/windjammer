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

//! WDB-099 / WindjammerDB PRE dogfood: owned formals must not get `&arg` at call sites.
//!
//! After cold multipass regen, PRE emitted:
//!   `opt_run_cost_report(&ledger)` where formal is owned non-Copy `OptEconLedger`
//!   `opt_quiet_median_and_contended(&samples)` where formal is owned `Vec<u64>`
//! → E0308. Call sites must move owned args when the callee formal is Owned.
//!
//! Gate A: current `wj` multipass (linked crate) must stay green.
//! Gate B: if `.worktrees/wj-pre-ir/target/release/wj` exists, it must also stay green
//! (WindjammerDB dogfood compiler — live as of PRE `wj` 0.50.0).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
    // Tip demotes read-only formals to `&T` / `&Vec<T>` and call sites must match.
    // The PRE bug was owned formals + borrowed call sites (E0308) — Gate B keeps that repro.
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

/// Gate C: owned aggregate struct at suite call site (Wave1OptLiveClaims pattern).
/// Passing on main wj 0.50.0 (2026-08-17); keep live so PRE-style over-borrow cannot regress.
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

/// Gate B: WindjammerDB dogfood PRE binary (when present) must match tip ownership.
///
/// PRE `wj` 0.50.0 (2026-08-22) is green; keep live so dogfood cannot regress.
#[test]
fn wdb099_pre_ir_dogfood_wj_must_not_borrow_owned_formals() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pre = manifest
        .join("..")
        .join(".worktrees")
        .join("wj-pre-ir")
        .join("target")
        .join("release")
        .join("wj");
    if !pre.exists() {
        eprintln!(
            "skip wdb099 PRE gate: dogfood binary missing at {}",
            pre.display()
        );
        return;
    }

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    let (mod_wj, harness, consumer_src) = wdb099_sources();
    fs::write(src.join("mod.wj"), mod_wj).unwrap();
    fs::write(src.join("harness.wj"), harness).unwrap();
    fs::write(src.join("consumer.wj"), consumer_src).unwrap();

    let build = Command::new(&pre)
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
            "--library",
            "--no-generate-cargo-toml",
        ])
        .output()
        .expect("run PRE wj");
    assert!(
        build.status.success(),
        "PRE wj build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let consumer = fs::read_to_string(out.join("consumer.rs")).expect("consumer.rs");
    let harness = fs::read_to_string(out.join("harness.rs")).expect("harness.rs");
    // PRE dogfood historically kept owned formals while over-borrowing at call sites.
    assert_consumer_matches_harness_ownership(&harness, &consumer);
}
