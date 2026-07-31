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
))]

//! REGRESSION (dogfood):
//!
//! `string.len()` is codegen'd as `usize`, but helpers taking WJ `int` expect `i64`.
//! `int_to_dec(raw.len())` → expected `i64`, found `usize`.
//! Fixed via IR NumericCast when actual/expected integer bases differ (WJ `int` → I64).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn string_len_passed_to_int_formal_must_agree() {
    let source = r#"
fn int_to_dec(value: int) -> string {
    if value == 0 {
        return "0"
    }
    "1"
}

pub fn frame(body: string) -> string {
    let raw = body + ""
    let len = raw.len()
    "Content-Length: " + int_to_dec(len) + "\r\n\r\n" + raw
}

fn main() {
    let _ = frame("{\"a\":1}" + "")
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("int_to_dec(len as i64)")
            || generated.contains("let len = raw.len() as i64")
            || generated.contains("let len: i64 = "),
        "string.len() (usize) must not feed i64 formal bare (E0308). Got:\n{generated}"
    );
    let check = test_utils::verify_rust_compiles(&generated);
    assert!(
        check.is_ok(),
        "Content-Length framing must rustc (MCP unlock). stderr={:?}\nGot:\n{generated}",
        check.err()
    );
}
