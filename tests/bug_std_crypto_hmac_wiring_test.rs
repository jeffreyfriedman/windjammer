//! `std::crypto` HMAC-SHA-256 and constant-time comparison must reach runtime crypto and `cargo check`.

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
fn std_crypto_hmac_and_constant_time_codegen_resolves() {
    let source = r#"
use std::crypto

pub fn sign(key: string, body: string) -> string {
    crypto.hmac_sha256(key, body)
}

pub fn sign_bytes(key: Vec<u8>, body: Vec<u8>) -> Vec<u8> {
    crypto.hmac_sha256_bytes(key, body)
}

pub fn check(key: string, body: string, tag: string) -> bool {
    crypto.verify_hmac_sha256(key, body, tag) && crypto.constant_time_eq(tag, tag)
}
"#;
    test_utils::assert_stdlib_runtime_links(
        source,
        &["crypto::hmac_sha256", "crypto::hmac_sha256_bytes", "crypto::verify_hmac_sha256", "crypto::constant_time_eq"],
    );
}
