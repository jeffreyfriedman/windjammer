//! `std::crypto.sha1_bytes` must exist for RFC 4122 UUID v5.

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
fn std_crypto_sha1_bytes_hashes_vec_u8() {
    let source = r#"
use std::crypto

pub fn digest(data: Vec<u8>) -> Vec<u8> {
    crypto.sha1_bytes(data)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["crypto::sha1_bytes"]);
}
