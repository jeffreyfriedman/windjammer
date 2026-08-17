//! HTTP status codes are `u16` (wire + runtime). Std stubs must not widen to `int`.

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn http_status_code_is_u16_not_i64() {
    let generated = test_utils::compile_single(
        r#"
use std::http

pub fn status_of(response: http.Response) -> u16 {
    response.status_code()
}
"#,
    );

    assert!(
        generated.contains("-> u16"),
        "expected u16 return in generated Rust:\n{generated}"
    );
    // Calling status_code into a u16 binding must not force `as i64`.
    let call_region = generated
        .find("status_code")
        .map(|i| &generated[i.saturating_sub(40)..(i + 80).min(generated.len())])
        .unwrap_or("");
    assert!(
        !call_region.contains("as i64"),
        "status_code must stay u16 (no widen-to-i64):\n{generated}"
    );
}
