//! `std::encoding.base64_encode_string` / `base64_decode_string` must `cargo check`.
//!
//! Platform encoding implements string variants; top-level `windjammer_runtime::encoding`
//! must re-export them (today only byte APIs are exported).

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
    test_utils::assert_stdlib_runtime_links(source, &["encoding::base64_encode_string"]);
}

#[test]
fn encoding_base64_decode_string_codegen_resolves() {
    let source = r#"
use std::encoding

pub fn decode_text(text: string) -> Result<string, string> {
    encoding.base64_decode_string(text)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["encoding::base64_decode_string"]);
}
