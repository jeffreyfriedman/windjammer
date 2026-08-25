//! Module `const string` passed to an owned `string` formal must auto-own.
//!
//! Ecosystem `wj-uuid`:
//! ```
//! pub const NAMESPACE_DNS: string = "6ba7b810-…"
//! pub fn v5_dns(name: string) -> Result<string, string> {
//!     v5(NAMESPACE_DNS, name)
//! }
//! ```
//! Const codegen as `&str` → E0308 vs `String` formal (related to
//! `bug_module_const_string_return_test`, but this gates the *call-site*
//! use of a module const into an owned formal).

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
fn module_const_string_into_owned_formal_auto_owns() {
    let source = r#"
pub const NAMESPACE: string = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"

pub fn take_owned(ns: string) -> string {
    ns
}

pub fn use_const() -> string {
    take_owned(NAMESPACE)
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "module const string into owned formal must compile, got:\n{generated}"
    );
}
