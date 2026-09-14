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

//! WDB-211: `fill_bundle_from_six` must not demote fills to `&mut OptOperatorFill`.
//!
//! Product residual (~14×), gen live lag vs tip-out:
//!   gen: `q1: &mut OptOperatorFill` + bare into owned `opt_operator_fill_is_complete`
//!   tip: owned fills + `.clone()` into is_complete. Prefer tip-out → gen sync.

use std::path::PathBuf;

#[test]
fn wdb211_product_live_must_not_demote_fill_bundle_to_mut_ref() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_opt_live_port.rs"),
        gen.join("relational/wave1_opt_live_port.rs"),
        gen.join("relational_module_file/wave1_opt_live_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("live");
        let bad = text.contains("fill_bundle_from_six(q1: &mut OptOperatorFill")
            || text.contains("q1: &mut OptOperatorFill, q6: &mut OptOperatorFill");
        eprintln!("WDB-211 bad={} path={}", bad, path.display());
        if path.to_string_lossy().contains("/gen/") {
            assert!(
                !bad,
                "WDB-211 RED: product gen demotes fill_bundle_from_six to &mut OptOperatorFill. {}",
                path.display()
            );
        }
    }
    assert!(saw, "WDB-211: live_port missing");
}
