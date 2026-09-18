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

//! WDB-272: `&owned.clone()` into `String::push_str` (`&str`) must reborrow (wave1).
//!
//! Twin of WDB-270. Tip-out wave1_publish still emits:
//!   `s.push_str(&dated_label.clone())`
//! while `push_str` takes `&str` — should be `s.push_str(&dated_label)`.
//! Signature-driven: borrow without clone into `&str` receivers.

use std::path::PathBuf;

#[test]
fn wdb272_tip_out_wave1_must_reborrow_owned_clone_into_push_str() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");

    let paths = [
        tip.join("wave1_publish_port.rs"),
        gen.join("graph/wave1_publish_port.rs"),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("wave1_publish");
        let bad = text.contains("push_str(&dated_label.clone())");
        eprintln!("WDB-272 bad={} path={}", bad, path.display());
        if bad {
            any_bad = true;
            bad_path = path.display().to_string();
        }
    }
    assert!(saw, "WDB-272: wave1_publish_port missing");
    assert!(
        !any_bad,
        "WDB-272 RED: tip-out/product passes &owned.clone() into push_str(&str). {}",
        bad_path
    );
}
