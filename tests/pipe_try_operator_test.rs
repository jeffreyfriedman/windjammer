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

//! `|>?` (docs/design/pipe-operator.md, "Short-circuiting stage"): `x |>? stage` is
//! `x? |> stage`, i.e. the stage is called with the unwrapped `Ok`/`Some` value and the
//! enclosing function returns early on `Err`/`None`. Expected trees are written by hand.

#[path = "common/pipe_ast.rs"]
mod pipe_ast;
use pipe_ast::sexp;

#[test]
fn try_pipe_unwraps_lhs_then_calls_stage() {
    assert_eq!(sexp("x |>? f").unwrap(), "(call f (try x))");
    assert_eq!(sexp("x |>? f(a)").unwrap(), "(call f (try x) a)");
    assert_eq!(sexp("x |>? f(a, _)").unwrap(), "(call f a (try x))");
    assert_eq!(sexp("x |>? .m(a)").unwrap(), "(meth (try x) m a)");
    assert_eq!(sexp("x |>? |v| v").unwrap(), "(call (fn [v] v) (try x))");
}

#[test]
fn try_pipe_chains_unwrap_each_stage_result() {
    assert_eq!(sexp("x |>? f |>? g").unwrap(), "(call g (try (call f (try x))))");
    assert_eq!(sexp("x |>? f |> g |>? h(1)").unwrap(), "(call h (try (call g (call f (try x)))) 1)");
}

#[test]
fn try_pipe_binds_looser_than_binary_operators() {
    assert_eq!(sexp("a + 1 |>? f").unwrap(), "(call f (try (Add a 1)))");
}

#[test]
fn try_pipe_mixes_with_postfix_try_on_stage() {
    assert_eq!(sexp("x |>? f()?").unwrap(), "(try (call f (try x)))");
}

#[test]
fn try_pipe_missing_stage_is_error_at_operator() {
    let src = "fn t() {\n    let r = x |>?\n}\n";
    let e = pipe_ast::parse_program_err(src).unwrap_err();
    assert!(e.starts_with("t.wj:2:15: error: expected a stage after `|>?`"), "got {e}");
}

#[test]
fn try_pipe_at_expression_start_is_error() {
    let e = pipe_ast::parse_program_err("fn t() {\n    |>? f\n}\n").unwrap_err();
    assert!(e.starts_with("t.wj:2:5: error: `|>?` needs a left-hand value"), "got {e}");
}

#[test]
fn try_pipe_rhs_not_callable_points_at_stage() {
    let e = pipe_ast::parse_program_err("fn t() {\n    let r = x |>? 5\n}\n").unwrap_err();
    assert!(e.starts_with("t.wj:2:19: error: right-hand side of `|>?`"), "got {e}");
}

#[test]
fn spaced_question_is_not_try_pipe() {
    // `|> ?` stays an error: the operator is the single token `|>?`.
    assert!(sexp("x |> ? f").is_err());
}
