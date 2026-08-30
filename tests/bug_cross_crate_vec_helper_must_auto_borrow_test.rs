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

//! FAILING REPRO — multipass demotes `Vec<string>` helper to `&Vec<String>` but call sites
//! emit owned `.clone()` (ecosystem `wj-cors` / `wj-migrate-cli` class).
//!
//! Pattern: sibling module passes bare `argv` (demotion) while another passes `argv.clone()`.
//! rustc E0308 when formal is `&Vec<String>` but call site passes `Vec<String>`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const CLI_ARGS: &str = r#"
pub fn first_positional(args: Vec<string>) -> Option<string> {
    if args.len() == 0 {
        return None
    }
    Some(args[0])
}
"#;

const DEMOTE_CALLER: &str = r#"
use crate::cli_args::first_positional

/// Bare `argv` reuse demotes helper formal in multipass.
pub fn peek(argv: Vec<string>) -> Option<string> {
    first_positional(argv)
}
"#;

const CLONE_CALLER: &str = r#"
use crate::cli_args::first_positional

pub fn use_positional(argv: Vec<string>) -> Option<string> {
    first_positional(argv.clone())
}
"#;

fn cross_crate_vec_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod cli_args
pub mod cli_demote
pub mod migrate_cli
"#,
    );
    test.add_file("cli_args.wj", CLI_ARGS);
    test.add_file("cli_demote.wj", DEMOTE_CALLER);
    test.add_file("migrate_cli.wj", CLONE_CALLER);
    test
}

#[test]
fn cross_crate_vec_string_helper_must_auto_borrow_at_call_site() {
    let mut test = cross_crate_vec_fixture();
    let map = test
        .compile()
        .expect("cross-module Vec helper compile should succeed");
    let helper = map
        .get("cli_args.rs")
        .expect("cli_args.rs must be generated");
    let clone_caller = map
        .get("migrate_cli.rs")
        .expect("migrate_cli.rs must be generated");

    let demoted = helper.contains("args: &Vec<String>") || helper.contains("args: &Vec <String>");
    let bad_clone_emit = clone_caller.contains("first_positional(argv.clone())")
        || clone_caller.contains("first_positional( argv.clone()");

    if demoted {
        assert!(
            !bad_clone_emit,
            "RED: demoted &Vec + owned .clone() call sites must borrow. Got:\n{clone_caller}"
        );
        let borrowed = clone_caller.contains("first_positional(&argv")
            || clone_caller.contains("first_positional(& argv");
        assert!(
            borrowed,
            "when multipass demotes to &Vec, .clone() call sites must borrow. Got:\n{clone_caller}"
        );
        test.cargo_check()
            .expect("borrowed Vec call sites must cargo-check");
    } else {
        assert!(
            helper.contains("args: Vec<String>"),
            "owned Vec formal is acceptable when multipass does not demote:\n{helper}"
        );
        test.cargo_check()
            .expect("owned Vec formal must cargo-check");
    }
}
