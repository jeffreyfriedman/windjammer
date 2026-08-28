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

//! FAILING REPRO — app same-module forwarder into cross-crate owned `String` formal
//! must move, not borrow.
//!
//! Ecosystem `wj-auth-api` pattern:
//! ```wj
//! fn hash_pw(password: string) -> Result<string, string> {
//!     let password = own(password)
//!     hash_password(password)  // wj_hash: owned String formal
//! }
//! ```
//! Multipass emits `hash_password(&password)` (E0308 expected `String`, found `&String`).
//! Inverse: `verify(token, secret)` demotes to `&str` but emits `.to_string()` at call site.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn cross_crate_owned_forwarder_must_move_local_not_borrow() {
    let generated = test_utils::compile_single(
        r#"
pub fn hash_password(password: string) -> Result<string, string> {
    Ok(password)
}

fn own(value: string) -> string {
    value
}

fn hash_pw(password: string) -> Result<string, string> {
    let password = own(password)
    hash_password(password)
}
"#,
    );

    assert!(
        !generated.contains("hash_password(&password)"),
        "owned cross-crate formal must receive moved String, not &String:\n{generated}"
    );
}

#[test]
fn cross_crate_borrowed_forwarder_must_auto_borrow_not_to_string() {
    let generated = test_utils::compile_single(
        r#"
pub fn verify(token: string, secret: string) -> Result<string, string> {
    Ok(token)
}

fn verify_jwt(token: string, secret: string) -> Result<string, string> {
    verify(token, secret)
}
"#,
    );

    assert!(
        !generated.contains("verify(token.to_string()"),
        "borrowed cross-crate formals must auto-borrow, not emit .to_string():\n{generated}"
    );
}
