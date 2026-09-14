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

//! WDB-210: `&mut Wave1Sf1Session` into owned `sess: Wave1Sf1Session` must clone/move.
//!
//! Product residual (~2×), tip-out/gen quiet_run:
//!   `wave1_sf1_clock_wdb_q6_sql_session(..., &mut sess)` while formal is owned Session.
//! Signature-driven.

use std::path::PathBuf;

#[test]
fn wdb210_tip_out_quiet_run_must_not_pass_mut_ref_session_into_owned_clock() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_sf1_quiet_run_port.rs"),
        gen.join("relational/wave1_sf1_quiet_run_port.rs"),
        gen.join("relational_module_file/wave1_sf1_quiet_run_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("quiet");
        let bad = text.contains("wave1_sf1_clock_wdb_q6_sql_session(")
            && text.contains("&mut sess")
            && !text.contains("sess.clone()")
            && text.contains("sess: Wave1Sf1Session");
        eprintln!("WDB-210 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-210 RED: tip-out/product passes &mut sess into owned Wave1Sf1Session clock. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-210: quiet_run missing");
}
