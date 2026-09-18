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

//! WDB-287: demoted `&line`/`&ord` into owned `wave1_sf1_session_from_batches` must clone.
//!
//! Twin of WDB-241/285. Tip-out still emits:
//!   `wave1_sf1_session_from_batches(qtys, &line, &ord, …)`
//! while formals are owned `Vec<i64>` → E0308.

use std::path::PathBuf;

#[test]
fn wdb287_tip_out_wave1_must_clone_ref_vec_into_owned_session_from_batches() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_sf1_session_port.rs"),
        gen.join("graph/wave1_sf1_session_port.rs"),
    ];
    let mut owned = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("wave1");
        if text.contains("fn wave1_sf1_session_from_batches(qtys: Vec<i64>, line_keys: Vec<i64>") {
            owned = true;
            break;
        }
    }
    assert!(owned, "WDB-287: owned Vec wave1_sf1_session_from_batches formals missing");

    let engine_paths = if tip.join("wave1_sf1_session_port.rs").exists() {
        vec![tip.join("wave1_sf1_session_port.rs")]
    } else {
        vec![gen.join("graph/wave1_sf1_session_port.rs")]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("wave1");
        let bad = text.contains("wave1_sf1_session_from_batches(")
            && (text.contains(", &line,")
                || text.contains(", &pair.1,")
                || text.contains(", &ord,"));
        eprintln!("WDB-287 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-287 RED: tip-out/product passes &Vec into owned wave1_sf1_session_from_batches. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-287: wave1_sf1_session_port missing");
}
