#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! WDB-160: `process::exit(2)` / integer exit codes must emit `i32`, not `i64`.
//!
//! WindjammerDB CQ-C5 cold tip gen (`wave1_artifact_cli.rs`):
//!   `process::exit(2_i64)` → E0308 expected `i32`, found `i64`
//! (~78 int-width residuals in post-cold census; exit is a sharp product hit).
//!
//! Literal `2` in exit position must unify with `i32` (Rust `process::exit` signature).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod cli
"#;

const CLI: &str = r#"
use std::process

pub fn exit_bad() {
    process::exit(2)
}

pub fn exit_code(code: int) {
    process::exit(code)
}
"#;

fn wdb160_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("cli.wj", CLI);
    test
}

#[test]
fn wdb160_module_file_process_exit_must_emit_i32_not_i64() {
    let mut test = wdb160_fixture();
    let map = test
        .compile()
        .expect("WDB-160 multipass compile should succeed (codegen may still be wrong)");
    let cli_rs = map.get("cli.rs").expect("cli.rs");

    eprintln!("WDB-160 cli.rs:\n{cli_rs}");

    let bad_lit = cli_rs.contains("process::exit(2_i64)")
        || cli_rs.contains("exit(2_i64)")
        || cli_rs.contains("exit(2 as i64)");
    let bad_param = cli_rs.contains("code: i64") && cli_rs.contains("process::exit(code)");

    if bad_lit || bad_param {
        panic!(
            "WDB-160 RED: process::exit must receive i32 (not i64). Product: wave1_artifact_cli process::exit(2_i64)."
        );
    }

    test.cargo_check().expect(
        "WDB-160: process::exit call sites must cargo-check as i32. Product: wave1_artifact_cli.",
    );
}
