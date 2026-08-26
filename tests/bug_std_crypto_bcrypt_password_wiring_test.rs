//! `std::crypto.hash_password` / `verify_password` must reach runtime bcrypt and `cargo check`.
//!
//! Ecosystem `wj-hash` wraps these for auth helpers.

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
fn std_crypto_hash_password_codegen_resolves() {
    let source = r#"
use std::crypto

pub fn hash_password(password: string) -> Result<string, string> {
    crypto.hash_password(password)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["crypto::hash_password"]);
}

#[test]
fn std_crypto_verify_password_codegen_resolves() {
    let source = r#"
use std::crypto

pub fn verify_password(password: string, hash: string) -> Result<bool, string> {
    crypto.verify_password(password, hash)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["crypto::verify_password"]);
}
