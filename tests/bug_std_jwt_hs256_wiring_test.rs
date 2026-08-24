//! `std::jwt` HS256 must codegen to `windjammer_runtime::jwt` and `cargo check`.

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
fn std_jwt_sign_hs256_codegen_resolves() {
    let source = r#"
use std::jwt

pub fn mint(secret: string) -> Result<string, string> {
    jwt.sign_hs256("user-1", "acme", secret, 3600)
}
"#;
    let generated = test_utils::assert_stdlib_runtime_links(source, &["jwt::sign_hs256"]);
    assert!(
        !generated.contains("import std::jwt"),
        "must not leave stub Err string as implementation, got:\n{generated}"
    );
}

#[test]
fn std_jwt_verify_hs256_codegen_resolves() {
    let source = r#"
use std::jwt

pub fn check(token: string, secret: string) -> Result<string, string> {
    match jwt.verify_hs256(token, secret) {
        Ok(claims) => Ok(claims.tenant_slug),
        Err(e) => Err(e),
    }
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["jwt::verify_hs256"]);
}
