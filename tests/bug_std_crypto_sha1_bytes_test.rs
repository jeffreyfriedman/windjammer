//! `std::crypto.sha1_bytes` must exist for RFC 4122 UUID v5 (namespace + name hashing).
//!
//! Ecosystem `wj-uuid`: `v5` / `v5_dns` / `v5_url`. SHA-256 is not a substitute.

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
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("crypto::sha1_bytes")
            || generated.contains("crypto::sha1_bytes("),
        "std::crypto.sha1_bytes must codegen to runtime SHA-1, got:\n{generated}"
    );
    assert!(
        !generated.contains("cannot find function `sha1_bytes`"),
        "must not emit missing sha1_bytes, got:\n{generated}"
    );
}
