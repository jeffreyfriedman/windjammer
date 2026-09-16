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

//! WDB-228: `string` formal must not emit `impl Into<String>` then `push_str(&param)`.
//!
//! Product residual (~9× &str←&impl Into), tip-out/gen lsqb_typed_graph:
//!   `lsqb_adj_out_key(edge_kind: impl Into<String>, …) { key.push_str(&edge_kind); }`
//! → E0308 expected `&str`, found `&impl Into<String>`.
//! Distinct from WDB-157 (clone on Into). Emit `String`/`&str` and `push_str` correctly.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/wdb228_string_formal_must_not_emit_impl_into_for_push_str.wj"
);

#[test]
fn wdb228_codegen_string_formal_must_not_emit_impl_into_for_push_str() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let into_formal = rs.contains("impl Into<String>") || rs.contains("impl Into<string>");
    let bad_push = rs.contains("push_str(&edge_kind)") && into_formal;
    if into_formal || bad_push {
        panic!(
            "WDB-228 RED: string formal emitted impl Into / push_str(&Into). Generated:\n{rs}"
        );
    }
    assert!(ok, "WDB-228 fixture must cargo-check. Generated:\n{rs}");
}

use std::path::PathBuf;

#[test]
fn wdb228_tip_out_lsqb_must_not_push_str_impl_into_edge_kind() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("lsqb_typed_graph.rs"),
        gen.join("graph/lsqb_typed_graph.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("lsqb");
        let into_formal = text.contains("edge_kind: impl Into<String>");
        let bad = into_formal && text.contains("push_str(&edge_kind)");
        eprintln!(
            "WDB-228 into_formal={} bad={} path={}",
            into_formal,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-228 RED: tip-out/product push_str(&impl Into) on edge_kind. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-228: lsqb_typed_graph missing");
}
