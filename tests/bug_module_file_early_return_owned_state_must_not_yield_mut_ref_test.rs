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

//! `&mut State` formal returning `(state, …)` owned tuple is invalid.
//!
//! Gate A: minimal early-return owned state must cargo-check.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("serve.wj", SERVE);
    test
}

#[test]
fn module_file_early_return_owned_state_must_not_yield_mut_ref() {
    let mut test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let serve_rs = map.get("serve.rs").expect("serve.rs");

    eprintln!("minimal serve.rs:\n{serve_rs}");

    assert!(
        !serve_rs.contains("state: &mut ServeState"),
        "minimal serve_once must keep owned ServeState formal."
    );

    test.cargo_check().expect(
        "early-return owned ServeState tuples must cargo-check.",
    );
}
