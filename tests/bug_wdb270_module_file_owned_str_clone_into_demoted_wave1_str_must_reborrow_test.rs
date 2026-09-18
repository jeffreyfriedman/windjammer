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
    feature = "codegen_tests",
))]

//! WDB-270: `&owned.clone()` into demoted `&str` wave1 helpers must reborrow.
//!
//! Tip demotes:
//!   `wave1_opt_hardware_dated_label_is_set(label: &str)`
//!   `wave1_opt_hardware_clock_postgres_sql(sql: &str)`
//! but tip-out still emits:
//!   `…_is_set(&dated_label.clone())` / `…_sql(&sql.clone())`
//! Signature-driven: pass `&dated_label` / `&sql` (no clone into `&str`).

use std::path::PathBuf;

#[test]
fn wdb270_tip_out_wave1_must_reborrow_owned_clone_into_demoted_str() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");

    let formal_paths = [
        tip.join("wave1_opt_hardware_report_port.rs"),
        tip.join("wave1_opt_hardware_port.rs"),
        gen.join("graph/wave1_opt_hardware_report_port.rs"),
        gen.join("graph/wave1_opt_hardware_port.rs"),
    ];
    let mut demoted = false;
    for path in &formal_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("wave1 formal");
        if text.contains("fn wave1_opt_hardware_dated_label_is_set(label: &str")
            || text.contains("fn wave1_opt_hardware_clock_postgres_sql(sql: &str")
        {
            demoted = true;
            break;
        }
    }
    assert!(
        demoted,
        "WDB-270: demoted &str wave1 dated_label_is_set / clock_postgres_sql formal missing"
    );

    let call_paths = [
        tip.join("wave1_publish_port.rs"),
        tip.join("wave1_sf1_quiet_run_port.rs"),
        gen.join("graph/wave1_publish_port.rs"),
        gen.join("graph/wave1_sf1_quiet_run_port.rs"),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for path in &call_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("wave1 call");
        let bad = text.contains("wave1_opt_hardware_dated_label_is_set(&dated_label.clone()")
            || text.contains("wave1_opt_hardware_clock_postgres_sql(&sql.clone()");
        eprintln!("WDB-270 bad={} path={}", bad, path.display());
        if bad {
            any_bad = true;
            bad_path = path.display().to_string();
        }
    }
    assert!(saw, "WDB-270: wave1 publish / sf1 quiet ports missing");
    assert!(
        !any_bad,
        "WDB-270 RED: tip-out/product passes &owned.clone() into demoted &str wave1 helpers. {}",
        bad_path
    );
}
