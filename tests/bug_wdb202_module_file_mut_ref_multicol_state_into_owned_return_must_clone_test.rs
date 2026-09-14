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

//! WDB-202: `&mut MulticolState` into owned return/tuple must clone/move.
//!
//! Product residual (~2×), gen sf1_multicol_serve:
//!   early return `(state, Vec::new(), false)` while `state` is `&mut …State`
//! → expected owned state, found `&mut`. Signature-driven.

use std::path::PathBuf;

#[test]
fn wdb202_product_multicol_must_not_return_mut_ref_state() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let serve = if tip
        .join("relational_pg_serve_sf1_multicol_serve_port.rs")
        .exists()
    {
        tip.join("relational_pg_serve_sf1_multicol_serve_port.rs")
    } else {
        gen.join("relational/relational_pg_serve_sf1_multicol_serve_port.rs")
    };
    // Always also check gen (product source of truth for this residual).
    let gen_serve = gen.join("relational/relational_pg_serve_sf1_multicol_serve_port.rs");
    for path in [serve, gen_serve] {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("multicol");
        let demoted = text.contains("state: &mut PgWireServeSf1MulticolState")
            || text.contains("state:&mut PgWireServeSf1MulticolState");
        let bad = demoted
            && text.contains("(state, Vec::new(), false)")
            && !text.contains("(state.clone(), Vec::new(), false)")
            && !text.contains("((*state).clone(), Vec::new(), false)");
        eprintln!(
            "WDB-202 demoted={} bad={} path={}",
            demoted,
            bad,
            path.display()
        );
        if path.to_string_lossy().contains("/gen/") {
            assert!(
                !bad,
                "WDB-202 RED: product gen returns &mut state where owned MulticolState required. {}",
                path.display()
            );
        }
    }
}
