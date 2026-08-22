//! `std::encoding.base64_encode_string` / `base64_decode_string` must resolve at codegen.
//!
//! Ecosystem `wj-base64` and `std/encoding/mod.wj` declare these APIs. Codegen emits
//! `encoding::base64_encode_string` but `windjammer_runtime::encoding` only exports
//! `base64_encode(&[u8])` / `base64_decode(&str)` (E0425). Platform native encoding
//! already implements the string variants — they must be wired through.

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
fn encoding_base64_encode_string_codegen_resolves() {
    let source = r#"
use std::encoding

pub fn encode_text(text: string) -> string {
    encoding.base64_encode_string(text)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("base64_encode_string"),
        "must call base64_encode_string, got:\n{generated}"
    );
    // Prefer a path that exists: runtime re-export or platform module.
    let wired = generated.contains("encoding::base64_encode_string")
        || generated.contains("windjammer_runtime::encoding::base64_encode_string");
    assert!(wired, "must emit encoding base64_encode_string call, got:\n{generated}");
}

#[test]
fn encoding_base64_decode_string_codegen_resolves() {
    let source = r#"
use std::encoding

pub fn decode_text(text: string) -> Result<string, string> {
    encoding.base64_decode_string(text)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("base64_decode_string"),
        "must call base64_decode_string, got:\n{generated}"
    );
}
