//! `use std::process` + `process.exit(1)` must emit `process::exit(1)`, not `process.exit(1)`.

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn process_exit_uses_path_separator() {
    let generated = test_utils::compile_single(
        r#"
use std::process

fn main() {
    process.exit(1)
}
"#,
    );

    assert!(
        generated.contains("process::exit(1)") || generated.contains("process::exit(1);"),
        "expected process::exit path call:\n{generated}"
    );
    assert!(
        !generated.contains("process.exit("),
        "must not emit method-style process.exit:\n{generated}"
    );
}
