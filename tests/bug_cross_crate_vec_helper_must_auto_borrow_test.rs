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

//! FAILING REPRO — app calling cross-crate `Vec<string>` helper must auto-borrow at call site.
//!
//! Ecosystem `wj-migrate-cli` pattern (blocked dogfood of `wj-cli-args`):
//! ```wj
//! use wj_cli_args::first_positional
//!
//! pub fn parse_apply(argv: Vec<string>) -> Result<string, string> {
//!     match first_positional(argv) { ... }
//! }
//! ```
//! Multipass emits `first_positional(argv.clone())` but the cross-crate Rust formal is
//! `&Vec<String>` → E0308 expected `&Vec<String>`, found `Vec<String>`.
//! Workaround: duplicate argv flag parsing locally (same as `wj-todo-cli`).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn cross_crate_vec_string_helper_must_auto_borrow_at_call_site() {
    let generated = test_utils::compile_single(
        r#"
pub fn first_positional(args: Vec<string>) -> Option<string> {
    if args.len() == 0 {
        return None
    }
    Some(args[0])
}

pub fn use_positional(argv: Vec<string>) -> Option<string> {
    first_positional(argv)
}
"#,
    );

    assert!(
        generated.contains("first_positional(&argv") || generated.contains("first_positional(& args"),
        "cross-crate Vec formal must receive borrowed Vec at call site:\n{generated}"
    );
    assert!(
        !generated.contains("first_positional(argv.clone())"),
        "must not pass owned Vec where cross-crate formal expects borrow:\n{generated}"
    );
}
