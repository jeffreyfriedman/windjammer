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

//! WDB-225: string lit into demoted `&str` must not emit `String::from` (join_path).
//!
//! Product residual (~46× &str←String), tip-out/gen graph_ldbc_validation_port:
//!   `join_path(root: &str, name: &str)` called with `String::from("BFS")` etc.
//! → E0308 expected `&str`, found `String`.
//! Twin of WDB-168 (pg_wire_parse); tip-out product gate for LDBC validation paths.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/wdb225_string_lit_into_demoted_str_must_not_emit_string_from.wj"
);

#[test]
fn wdb225_codegen_string_lit_into_demoted_str_must_not_emit_string_from() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let demoted = rs.contains("fn join_path(root: &str")
        || rs.contains("name: &str")
        || (rs.contains("join_path") && rs.contains("&str"));
    let bad = rs.contains("String::from(\"BFS\")")
        && (rs.contains("join_path(root, String::from")
            || rs.contains("join_path(root,String::from"));
    if demoted && bad {
        panic!(
            "WDB-225 RED: demoted &str received String::from(\"BFS\"). Generated:\n{rs}"
        );
    }
    assert!(ok, "WDB-225 fixture must cargo-check. Generated:\n{rs}");
}

use std::path::PathBuf;

#[test]
fn wdb225_tip_out_ldbc_must_not_pass_string_from_into_demoted_join_path() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_ldbc_validation_port.rs"),
        gen.join("graph/graph_ldbc_validation_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("ldbc");
        let demoted = text.contains("fn join_path(root: &str, name: &str)");
        let bad = text.contains("join_path(validation_root, String::from(\"BFS\")")
            || text.contains("join_path(validation_root, String::from(\"PR\")");
        eprintln!(
            "WDB-225 demoted={} bad={} path={}",
            demoted,
            bad,
            path.display()
        );
        if demoted {
            assert!(
                !bad,
                "WDB-225 RED: tip-out/product passes String::from into demoted &str join_path. {}",
                path.display()
            );
        }
    }
    assert!(saw, "WDB-225: graph_ldbc_validation_port missing");
}
