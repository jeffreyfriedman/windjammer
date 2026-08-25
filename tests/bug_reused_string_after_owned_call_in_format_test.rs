//! Owned `string` formal must not prevent later use of the same local.
//!
//! Ecosystem `wj-multipart` tests:
//! ```
//! let out = format_multipart(boundary, parts)
//! assert(strings.contains(out, "--${boundary}--"), "…")
//! ```
//! Codegen moves `boundary` into the call, then `${boundary}` in the format
//! string fails with E0382. Signature-driven borrow (read-only formal) or
//! auto-clone at the later use site should keep idiomatic WJ compiling.

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
fn reused_string_after_owned_helper_call_compiles() {
    let source = r#"
use std::strings

pub fn take_boundary(boundary: string) -> string {
    boundary
}

pub fn check(boundary: string) -> bool {
    let out = take_boundary(boundary)
    strings.contains(out, "--${boundary}--")
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "reusing string after owned formal call must compile, got:\n{generated}"
    );
}
