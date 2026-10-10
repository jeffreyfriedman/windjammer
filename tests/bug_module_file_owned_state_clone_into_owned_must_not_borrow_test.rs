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

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("serve.wj", SERVE);
    test.add_file("feed.wj", FEED);
    test
}

#[test]
fn module_file_owned_state_clone_into_owned_must_not_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let serve_rs = map.get("serve.rs").expect("serve.rs");
    let feed_rs = map.get("feed.rs").expect("feed.rs");

    eprintln!("serve.rs:\n{serve_rs}\nfeed.rs:\n{feed_rs}");

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
            "owned state.clone() into owned formal got leading &. \
             Product: pg_wire_serve_on_parse(&state.clone(), …). Got:\n{feed_rs}\n{serve_rs}"
        );
    }

    if callee_owned {
        assert!(
            good && !bad,
            "owned formal must receive state.clone() not &state.clone(). Got:\n{feed_rs}"
        );
    }
}
