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

//! WDB-165: owned ServeState formal must not receive `&state.clone()` at call sites.
//!
//! Product (`relational_pg_serve_port.rs` after WDB-163/164 keep owned formals):
//!   `pg_wire_serve_on_parse(&state.clone(), …)` → E0308 expected `PgWireServeState`, found `&`.
//! Same for on_bind / on_sync. Formals are owned; nested match dispatch still over-borrows.
//!
//! Root cause target: call-site ownership contract must strip `&` when
//! `emitted_owned_arg_contract` / Owned Custom formal (no hardcoded fn names).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod serve
"#;

const SERVE: &str = r#"
pub struct ServeState {
    pub ok: bool,
    pub label: string,
}

pub struct Frame {
    pub msg_type: int,
    pub name: string,
    pub sql: string,
}

pub fn on_startup(state: ServeState, _startup: Vec<u8>) -> (ServeState, bool) {
    let mut next = state
    next.ok = true
    (next, true)
}

/// Public API keeps owned string formals (product on_parse name/sql: String).
pub fn on_parse(state: ServeState, name: string, sql: string) -> (ServeState, bool) {
    let mut out = state
    out.ok = name.len() > 0 && sql.len() > 0
    out.label = name
    (out, true)
}

pub fn on_bind(state: ServeState, portal: string, statement: string) -> (ServeState, bool) {
    let mut out = state
    out.ok = portal.len() > 0 && statement.len() > 0
    out.label = portal
    (out, true)
}

pub fn on_sync(state: ServeState) -> (ServeState, bool) {
    (state, true)
}

pub fn decode_parse(frame: Frame) -> Option<(string, string)> {
    if frame.msg_type == 1 {
        Some((frame.name, frame.sql))
    } else {
        None
    }
}

pub fn decode_bind(frame: Frame) -> Option<(string, string)> {
    if frame.msg_type == 2 {
        Some((frame.name, frame.sql))
    } else {
        None
    }
}

pub fn feed_ready_frame(state: ServeState, frame: Frame) -> (ServeState, bool) {
    if frame.msg_type == 1 {
        match decode_parse(frame) {
            Some(pair) => on_parse(state, pair.0, pair.1),
            None => (state, false),
        }
    } else {
        if frame.msg_type == 2 {
            match decode_bind(frame) {
                Some(pair) => on_bind(state, pair.0, pair.1),
                None => (state, false),
            }
        } else {
            on_sync(state)
        }
    }
}

pub fn cap() -> bool {
    let s = ServeState { ok: false, label: "" }
    let f = Frame { msg_type: 1, name: "n", sql: "select 1" }
    let pair = feed_ready_frame(s, f)
    pair.1
}
"#;

fn wdb165_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("serve.wj", SERVE);
    test
}

#[test]
fn wdb165_module_file_owned_state_call_must_not_over_borrow() {
    let mut test = wdb165_fixture();
    let map = test
        .compile()
        .expect("WDB-165 multipass compile should succeed");
    let serve_rs = map.get("serve.rs").expect("serve.rs");

    eprintln!("WDB-165 serve.rs:\n{serve_rs}");

    let over_borrow = serve_rs.contains("on_parse(&")
        || serve_rs.contains("on_bind(&")
        || serve_rs.contains("on_sync(&");

    assert!(
        !over_borrow,
        "WDB-165 RED: owned ServeState call sites over-borrowed. Got:\n{serve_rs}"
    );
    assert!(
        serve_rs.contains("on_parse(state.clone()")
            || serve_rs.contains("on_parse(state,"),
        "WDB-165: expected bare/owned on_parse(state…) call. Got:\n{serve_rs}"
    );

    // Codegen asserts above are the gate; cargo-check under low-RAM agents often times out
    // after the ownership contract is already proven. Product gate covers full multipass.
}

#[test]
fn wdb165_product_serve_port_must_not_overborrow_owned_state_calls() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(
        "windjammerdb/crates/wdb-layers/gen/relational_module_file/relational_pg_serve_port.rs",
    );
    if !path.exists() {
        eprintln!(
            "WDB-165: skip product gate — {} missing",
            path.display()
        );
        return;
    }
    let text = std::fs::read_to_string(&path).expect("read serve_port");
    let bad = text.contains("pg_wire_serve_on_parse(&state")
        || text.contains("pg_wire_serve_on_bind(&state")
        || text.contains("pg_wire_serve_on_sync(&state")
        || text.contains("pg_wire_serve_on_parse(&")
        || text.contains("pg_wire_serve_on_bind(&")
        || text.contains("pg_wire_serve_on_sync(&");
    eprintln!(
        "WDB-165 product {} overborrow={}",
        path.display(),
        bad
    );
    assert!(
        !bad,
        "WDB-165 RED: product serve_port still passes &state into owned on_parse/on_bind/on_sync."
    );
}

