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

//! WDB-296: gen-lag — tip-out bare `"BFS"` into demoted `join_path` `&str` must stay,
//! but gen still emits `String::from("BFS")` (twin WDB-225 product site).
//!
//! Tip-out GREEN; gen RED (~6× LDBC validation). Prefer tip-out→gen sync once tip
//! greens related string-lit demotion; this gate fails until gen matches tip.

use std::path::PathBuf;

#[test]
fn wdb296_gen_must_not_lag_tip_bare_str_into_demoted_join_path() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen/graph/graph_ldbc_validation_port.rs");

    let tip_paths = [
        tip.join("graph_ldbc_validation_port.rs"),
        tip.join("graph/graph_ldbc_validation_port.rs"),
    ];
    let mut tip_ok = false;
    for path in &tip_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("tip ldbc");
        let demoted = text.contains("fn join_path(root: &str, name: &str)");
        let bare = text.contains("join_path(validation_root, \"BFS\")");
        let bad_from = text.contains("join_path(validation_root, String::from(\"BFS\")");
        eprintln!(
            "WDB-296 tip demoted={} bare={} bad_from={} path={}",
            demoted,
            bare,
            bad_from,
            path.display()
        );
        if demoted && bare && !bad_from {
            tip_ok = true;
            break;
        }
    }
    assert!(
        tip_ok,
        "WDB-296: tip-out should already emit bare \"BFS\" into demoted join_path"
    );

    assert!(gen.exists(), "WDB-296: gen graph_ldbc_validation_port missing");
    let text = std::fs::read_to_string(&gen).expect("gen ldbc");
    let demoted = text.contains("fn join_path(root: &str, name: &str)");
    let bad = demoted
        && text.contains("join_path(validation_root, String::from(\"BFS\")");
    eprintln!("WDB-296 gen demoted={} bad={} path={}", demoted, bad, gen.display());
    assert!(
        !bad,
        "WDB-296 RED: gen lags tip — still passes String::from into demoted join_path. {}",
        gen.display()
    );
}
