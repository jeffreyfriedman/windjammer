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

//! WDB-184: `state.clone()` into owned formal must not emit `&state.clone()`.
//!
//! Product residual (~6× PgWireServeState←&PgWireServeState), tip/product pg_serve:
//!   `pg_wire_serve_on_parse(state: PgWireServeState, …)`
//!   call site `pg_wire_serve_on_parse(&state.clone(), …)` → E0308.
//! Same class as WDB-169 (owned helper/temp into owned must not invent `&`).
//! Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod serve
pub mod feed
"#;

const SERVE: &str = r#"
pub struct ServeState {
    pub phase: int,
    pub snapshot: int,
}

/// Owned state consumer (product pg_wire_serve_on_parse).
pub fn on_parse(state: ServeState, name: string, sql: string) -> ServeState {
    let _ = name.len() + sql.len()
    ServeState { phase: state.phase + 1, snapshot: state.snapshot }
}

pub fn on_bind(state: ServeState, portal: string, statement: string) -> ServeState {
    let _ = portal.len() + statement.len()
    ServeState { phase: state.phase + 1, snapshot: state.snapshot }
}

pub fn on_sync(state: ServeState) -> ServeState {
    ServeState { phase: state.phase, snapshot: state.snapshot }
}
"#;

const FEED: &str = r#"
use crate::serve::ServeState
use crate::serve::on_parse
use crate::serve::on_bind
use crate::serve::on_sync

pub fn feed_ready_frame(state: ServeState, name: string, sql: string) -> ServeState {
    // Product: on_parse(&state.clone(), …) while formal owned — must state.clone()
    on_parse(state.clone(), name, sql)
}

pub fn feed_bind(state: ServeState, portal: string, statement: string) -> ServeState {
    on_bind(state.clone(), portal, statement)
}

pub fn feed_sync(state: ServeState) -> ServeState {
    on_sync(state.clone())
}
"#;

fn wdb184_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("serve.wj", SERVE);
    test.add_file("feed.wj", FEED);
    test
}

#[test]
fn wdb184_module_file_owned_state_clone_into_owned_must_not_borrow() {
    let test = wdb184_fixture();
    let map = test
        .compile()
        .expect("WDB-184 multipass compile should succeed");
    let serve_rs = map.get("serve.rs").expect("serve.rs");
    let feed_rs = map.get("feed.rs").expect("feed.rs");

    eprintln!("WDB-184 serve.rs:\n{serve_rs}\nfeed.rs:\n{feed_rs}");

    let callee_owned = {
        let i = serve_rs.find("fn on_parse").unwrap_or(0);
        let sl = &serve_rs[i..serve_rs.len().min(i + 120)];
        (sl.contains("state: ServeState") || sl.contains("state:ServeState"))
            && !(sl.contains("state: &ServeState") || sl.contains("state:&ServeState"))
    };
    let bad = feed_rs.contains("on_parse(&state.clone()")
        || feed_rs.contains("on_bind(&state.clone()")
        || feed_rs.contains("on_sync(&state.clone()");
    let good = feed_rs.contains("on_parse(state.clone()")
        || feed_rs.contains("on_bind(state.clone()")
        || feed_rs.contains("on_sync(state.clone()");

    if callee_owned && bad {
        panic!(
            "WDB-184 RED: owned state.clone() into owned formal got leading &. \
             Product: pg_wire_serve_on_parse(&state.clone(), …). Got:\n{feed_rs}\n{serve_rs}"
        );
    }

    if callee_owned {
        assert!(
            good && !bad,
            "WDB-184: owned formal must receive state.clone() not &state.clone(). Got:\n{feed_rs}"
        );
    }
}

#[test]
fn wdb184_product_pg_serve_must_not_borrow_state_clone_into_owned() {
    // Product residual lives in gitignored gen/ (tip-out may already be green —
    // artifact lag). Prefer gen; also fail if tip-out regresses.
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        gen.join("relational/relational_pg_serve_port.rs"),
        tip.join("relational_pg_serve_port.rs"),
    ];
    let mut saw = false;
    for serve in &paths {
        if !serve.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(serve).expect("serve");
        let parse_owned = text.contains("fn pg_wire_serve_on_parse(state: PgWireServeState");
        let bad = text.contains("pg_wire_serve_on_parse(&state.clone()")
            || text.contains("pg_wire_serve_on_bind(&state.clone()")
            || text.contains("pg_wire_serve_on_sync(&state.clone()")
            || text.contains("pg_wire_serve_feed_frames_at_offset(&state.clone()");
        eprintln!(
            "WDB-184 product parse_owned={} bad={} path={}",
            parse_owned,
            bad,
            serve.display()
        );
        // Only enforce on gen (product); tip-out green is progress but gen lag is RED.
        if serve.starts_with(&gen) {
            assert!(
                !(parse_owned && bad),
                "WDB-184 RED: product gen still passes &state.clone() into owned pg_wire_serve_*. {}",
                serve.display()
            );
        }
    }
    if !saw {
        eprintln!("WDB-184: skip product — pg_serve missing");
    }
}
