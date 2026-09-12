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

//! WDB-163: `&mut State` formal returning `(state, …)` owned tuple is invalid.
//!
//! Product `pg_wire_serve_on_simple_query(state: &mut PgWireServeState, …) -> (PgWireServeState, …)`
//! early-returns `(state, Vec::new(), false)` → E0308 expected owned, found `&mut` (~36).
//!
//! Gate A: minimal early-return owned state must cargo-check.
//! Gate B: product gen must not pair `&mut PgWireServeState` formal with bare `(state,` returns.

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
    pub ticks: int,
}

pub fn serve_once(state: ServeState, fail: bool) -> (ServeState, bool) {
    if fail {
        return (state, false)
    }
    let mut out = state
    out.ok = true
    out.ticks = out.ticks + 1
    (out, true)
}

pub fn cap() -> bool {
    let s = ServeState { ok: false, ticks: 0 }
    let pair = serve_once(s, true)
    !pair.1
}
"#;

fn wdb163_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("serve.wj", SERVE);
    test
}

#[test]
fn wdb163_module_file_early_return_owned_state_must_not_yield_mut_ref() {
    let mut test = wdb163_fixture();
    let map = test
        .compile()
        .expect("WDB-163 multipass compile should succeed (codegen may still be wrong)");
    let serve_rs = map.get("serve.rs").expect("serve.rs");

    eprintln!("WDB-163 minimal serve.rs:\n{serve_rs}");

    assert!(
        !serve_rs.contains("state: &mut ServeState"),
        "WDB-163: minimal serve_once must keep owned ServeState formal (product demotes to &mut)."
    );

    test.cargo_check().expect(
        "WDB-163: early-return owned ServeState tuples must cargo-check.",
    );
}

#[test]
fn wdb163_product_pg_wire_serve_mut_state_must_not_return_bare_state() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(
        "windjammerdb/crates/wdb-layers/gen/relational_module_file/relational_pg_serve_port.rs",
    );

    if !path.exists() {
        eprintln!(
            "WDB-163: skip product gate — {} missing (run transpile_relational_module_file.sh)",
            path.display()
        );
        return;
    }

    let text = std::fs::read_to_string(&path).expect("read serve_port gen");
    let has_mut_formal = text.contains("state: &mut PgWireServeState");
    let bare_return = text.contains("(state, Vec::new(), false)");
    eprintln!(
        "WDB-163 product {} mut_formal={} bare_return={}",
        path.display(),
        has_mut_formal,
        bare_return
    );

    if has_mut_formal && bare_return {
        panic!(
            "WDB-163 RED: full multipass &mut PgWireServeState formal early-returns bare (state, …). \
             Tip-cluster alone keeps owned formal. Product: relational_pg_serve_port."
        );
    }
}
