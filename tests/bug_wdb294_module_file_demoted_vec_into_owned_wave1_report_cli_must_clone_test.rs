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

//! WDB-294: demoted `&args` into owned `wave1_report_cli_is_report_command` must clone.
//!
//! Twin of WDB-291/292. Tip `relational/wave1_report_cli.rs` has owned
//! `args: Vec<String>` while dispatcher `wave1_cli.rs` still emits
//! `wave1_report_cli_is_report_command(&args)` → E0308.

use std::path::PathBuf;

#[test]
fn wdb294_tip_out_wave1_report_cli_must_clone_ref_vec_into_owned_is_report() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let formal_paths = [
        tip.join("relational/wave1_report_cli.rs"),
        tip.join("wave1_report_cli.rs"),
        gen.join("relational/wave1_report_cli.rs"),
    ];
    let mut owned = false;
    for path in &formal_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("report");
        if text.contains("fn wave1_report_cli_is_report_command(args: Vec<String>") {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-294: owned Vec wave1_report_cli_is_report_command formal missing"
    );

    let call_paths = [
        tip.join("wave1_cli.rs"),
        tip.join("relational/wave1_cli.rs"),
        gen.join("relational/wave1_cli.rs"),
    ];
    let mut saw = false;
    for path in &call_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("wave1_cli");
        let bad = text.contains("wave1_report_cli_is_report_command(&args)");
        eprintln!("WDB-294 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-294 RED: tip-out/product passes &Vec into owned wave1_report_cli_is_report_command. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-294: wave1_cli missing");
}
