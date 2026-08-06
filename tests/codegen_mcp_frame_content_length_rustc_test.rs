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

//! MCP / E3.9.x unlock: Content-Length framing must rustc.
//!
//! LedgerKit `mcp_frame_message` historically hit `string.len()` (usize) → `int`
//! formal (i64). Prefer NumericCast or `format!`; either path must cargo-check.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn mcp_frame_int_to_dec_len_must_rustc() {
    let source = r#"
fn int_to_dec(value: int) -> string {
    if value == 0 {
        return "0"
    }
    if value < 10 {
        return "9"
    }
    "42"
}

pub fn mcp_frame_message(body: string) -> string {
    let raw = body + ""
    let len = raw.len()
    "Content-Length: " + int_to_dec(len) + "\r\n\r\n" + raw
}

fn main() {
    let _ = mcp_frame_message("{\"jsonrpc\":\"2.0\"}" + "")
}
"#;
    let generated = test_utils::compile_single(source);
    let check = test_utils::verify_rust_compiles(&generated);
    assert!(
        check.is_ok(),
        "MCP Content-Length via int_to_dec(len) must rustc (usize→i64). stderr={:?}\nGot:\n{generated}",
        check.err()
    );
}

#[test]
fn mcp_frame_format_len_must_rustc() {
    let source = r#"
pub fn mcp_frame_message(body: string) -> string {
    let raw = body + ""
    format!("Content-Length: {}\r\n\r\n{}", raw.len(), raw)
}

fn main() {
    let _ = mcp_frame_message("{\"jsonrpc\":\"2.0\"}" + "")
}
"#;
    let generated = test_utils::compile_single(source);
    let check = test_utils::verify_rust_compiles(&generated);
    assert!(
        check.is_ok(),
        "MCP Content-Length via format!(len) must rustc. stderr={:?}\nGot:\n{generated}",
        check.err()
    );
}
