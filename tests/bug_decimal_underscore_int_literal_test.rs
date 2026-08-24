//! Decimal integer literals with `_` digit separators must parse and emit the
//! same value as the undashed form (Rust parity: `60_000` → `60000`).
//!
//! Ecosystem agents previously "worked around" by rewriting `60_000` → `60000`
//! while debugging unrelated ownership bugs. Lexer already skips `_`; this gate
//! locks the end-to-end transpile shape so the workaround does not return.

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn decimal_underscore_int_literal_emits_plain_value() {
    let source = r#"
pub fn window_ms() -> int {
    60_000
}

pub fn million() -> int {
    1_000_000
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("60000") || generated.contains("60_000"),
        "60_000 must emit 60000 (or keep separator), got:\n{generated}"
    );
    assert!(
        generated.contains("1000000") || generated.contains("1_000_000"),
        "1_000_000 must emit 1000000 (or keep separator), got:\n{generated}"
    );
    assert!(
        !generated.contains("compile_error!"),
        "underscore int literals must not fail-closed, got:\n{generated}"
    );
}
