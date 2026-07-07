#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

//! WJ-SYN-01: `x |> f(a, b)` desugars to `f(x, a, b)`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_pipe_inserts_first_argument_into_call() {
    let code = r#"
    fn combine(a: int, b: int, c: int) -> int {
        a + b + c
    }

    pub fn piped() -> int {
        5 |> combine(10, 20)
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    assert!(
        generated.contains("combine(5") && generated.contains("10") && generated.contains("20"),
        "Pipe with call should insert piped value as first arg. Got:\n{}",
        generated
    );
}
