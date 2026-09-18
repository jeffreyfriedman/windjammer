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

//! WDB-290: demoted `&args` into owned `wave1_sf1_cli_parse_sf1_floor` must clone.
//!
//! Twin of WDB-241/285. Tip relational emit:
//!   `wave1_sf1_cli_parse_sf1_floor(&args)` while formal is `args: Vec<String>` → E0308.

use std::path::PathBuf;

#[test]
fn wdb290_tip_out_wave1_sf1_cli_must_clone_ref_vec_into_owned_parse_sf1_floor() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("relational/wave1_sf1_cli.rs"),
        tip.join("wave1_sf1_cli.rs"),
        gen.join("relational/wave1_sf1_cli.rs"),
    ];
    let mut owned = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("wave1_sf1_cli");
        if text.contains("fn wave1_sf1_cli_parse_sf1_floor(args: Vec<String>") {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-290: owned Vec wave1_sf1_cli_parse_sf1_floor formal missing"
    );

    let engine_paths = if tip.join("relational/wave1_sf1_cli.rs").exists() {
        vec![tip.join("relational/wave1_sf1_cli.rs")]
    } else if tip.join("wave1_sf1_cli.rs").exists() {
        vec![tip.join("wave1_sf1_cli.rs")]
    } else {
        vec![gen.join("relational/wave1_sf1_cli.rs")]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("wave1_sf1_cli");
        let owned_here = text.contains("fn wave1_sf1_cli_parse_sf1_floor(args: Vec<String>");
        let bad = owned_here && text.contains("wave1_sf1_cli_parse_sf1_floor(&args)");
        eprintln!(
            "WDB-290 owned_here={} bad={} path={}",
            owned_here,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-290 RED: tip-out/product passes &Vec into owned wave1_sf1_cli_parse_sf1_floor. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-290: wave1_sf1_cli missing");
}
