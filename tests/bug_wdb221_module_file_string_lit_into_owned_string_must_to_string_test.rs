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

//! WDB-221: string literal into owned `String` formal must `.to_string()`.
//!
//! Product residual after tip regen (~298× String←&str), tip-out/gen lsqb:
//!   `lsqb_in_neighbors(graph, "Person_likes_Post", message)` while
//!   `edge_kind: String` → expected `String`, found `&str`.
//! Inverse of WDB-214 (owned into demoted `&str`). Signature-driven.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/wdb221_string_lit_into_owned_string_must_to_string.wj");

#[test]
fn wdb221_codegen_string_lit_into_owned_must_to_string() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let owned = rs.contains("fn needs_owned(kind: String") || rs.contains("needs_owned(kind: String");
    let bad = owned
        && rs.contains("needs_owned(\"Person_likes_Post\")")
        && !rs.contains("needs_owned(\"Person_likes_Post\".to_string())");
    eprintln!("WDB-221 codegen owned={} bad={}\n{rs}", owned, bad);
    assert!(
        !bad,
        "WDB-221: string lit into owned String must .to_string(). Generated:\n{rs}"
    );
    assert!(ok, "WDB-221 fixture must cargo-check. Generated:\n{rs}");
}

use std::path::PathBuf;

#[test]
fn wdb221_tip_out_lsqb_must_to_string_edge_kind_lits_into_owned() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let typed_paths = [
        tip.join("lsqb_typed_graph.rs"),
        gen.join("graph/lsqb_typed_graph.rs"),
    ];
    let mut typed = String::new();
    for path in &typed_paths {
        if path.exists() {
            typed = std::fs::read_to_string(path).expect("typed");
            break;
        }
    }
    assert!(!typed.is_empty(), "WDB-221: lsqb_typed_graph missing");
    let owned_kind = typed.contains("edge_kind: String");

    let engine_paths = [
        tip.join("lsqb_query_engine.rs"),
        gen.join("graph/lsqb_query_engine.rs"),
    ];
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("engine");
        // Bare string lit into neighbors (no .to_string()) while formal is owned String.
        let bare_lit = text.contains("lsqb_in_neighbors(graph, \"")
            || text.contains("lsqb_out_neighbors(graph, \"")
            || text.contains("lsqb_in_neighbors(graph.clone(), \"")
            || text.contains("lsqb_out_neighbors(graph.clone(), \"");
        let has_to_string = text.contains(".to_string(), message")
            || text.contains(".to_string(), comment")
            || text.contains(".to_string(), tag")
            || text.contains(".to_string(), p3")
            || text.contains("\"Person_likes_Post\".to_string()");
        let bad = owned_kind && bare_lit && !text.contains("\"Person_likes_Post\".to_string()");
        eprintln!(
            "WDB-221 owned_kind={} bare_lit={} has_to_string={} bad={} path={}",
            owned_kind,
            bare_lit,
            has_to_string,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-221 RED: tip-out/product passes string lit into owned edge_kind without .to_string(). {}",
            path.display()
        );
    }
    assert!(saw, "WDB-221: lsqb_query_engine missing");
}
