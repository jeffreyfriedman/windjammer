//! `std::crypto.sha256_hex` must reach runtime crypto and `cargo check`.

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
fn std_crypto_sha256_hex_codegen_resolves() {
    let source = r#"
use std::crypto

pub fn digest(text: string) -> string {
    crypto.sha256_hex(text)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["crypto::sha256_hex"]);
}
