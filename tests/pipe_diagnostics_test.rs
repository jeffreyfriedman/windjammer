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

//! Pipe diagnostics: every misuse is reported at the exact `file:line:col` (1-based) of the
//! offending token, with a message that names the problem. Columns are counted by hand from the
//! snippet position, not taken from the compiler.

#[path = "common/pipe_ast.rs"]
mod pipe_ast;

/// Source wrapper: the expression starts at line 2, column 13.
fn err(expr: &str) -> String {
    let src = format!("fn t() {{\n    let r = {expr}\n}}\n");
    pipe_ast::parse_program_err(&src).expect_err("expected a diagnostic")
}

/// Does `e` report `msg` at the position of the `nth` (0-based) occurrence of `needle` in `expr`?
fn reported_at(e: &str, expr: &str, needle: &str, nth: usize, msg: &str) -> bool {
    let idx = expr.match_indices(needle).nth(nth).expect("needle in expr").0;
    e.starts_with(&format!("t.wj:2:{}: error: {msg}", 13 + idx))
}

#[test]
fn second_placeholder_is_reported_at_the_second_underscore() {
    let x = "x |> f(_, _)";
    let e = err(x);
    assert!(reported_at(&e, x, "_", 1, "a pipe stage may contain at most one `_`"), "{e}");
    let x = "x |> f(a, _, b, _, _)";
    assert!(reported_at(&err(x), x, "_", 1, "a pipe stage may contain at most one `_`"));
}

#[test]
fn nested_placeholder_is_reported_at_that_underscore() {
    let x = "x |> g(1, f(_))";
    let e = err(x);
    assert!(reported_at(&e, x, "_", 0, "placeholder `_` must be a direct argument"), "{e}");
    let x = "x |> g(_, h(2, _))";
    assert!(reported_at(&err(x), x, "_", 1, "placeholder `_` must be a direct argument"));
}

#[test]
fn placeholder_outside_a_stage_is_reported_at_the_underscore() {
    for x in ["f(_)", "_ + 1", "a + _"] {
        let e = err(x);
        assert!(reported_at(&e, x, "_", 0, "placeholder `_` is only valid as a direct argument of a `|>` stage"), "{x}: {e}");
    }
    // The left-hand side of a pipe is not a stage.
    let x = "_ |> f";
    assert!(reported_at(&err(x), x, "_", 0, "placeholder `_` is only valid"));
}

#[test]
fn bare_placeholder_stage_is_rejected_at_the_underscore() {
    let x = "x |> _";
    let e = err(x);
    assert!(reported_at(&e, x, "_", 0, "placeholder `_` cannot be the whole stage"), "{e}");
}

#[test]
fn empty_right_hand_side_is_reported_at_the_operator() {
    for (x, op) in [("x |>", "|>"), ("x |>?", "|>?")] {
        let e = err(x);
        assert!(reported_at(&e, x, op, 0, &format!("expected a stage after `{op}`")), "{e}");
    }
    // Followed by another pipe / a closing bracket.
    let x = "f(x |>)";
    assert!(reported_at(&err(x), x, "|>", 0, "expected a stage after `|>`"));
    let x = "x |> |> f";
    assert!(reported_at(&err(x), x, "|>", 0, "expected a stage after `|>`"));
}

#[test]
fn non_callable_right_hand_sides_are_reported_at_the_stage() {
    for stage in ["5", "\"s\"", "true", "(1, 2)", "[1, 2]", "if c { f } else { g }", "-f"] {
        let x = format!("x |> {stage}");
        let e = err(&x);
        let ok = e.starts_with(&format!("t.wj:2:{}: error: ", 13 + 5)) && !e.is_empty();
        assert!(ok, "`{x}`: {e}");
    }
    let x = "x |> 5";
    assert!(reported_at(&err(x), x, "5", 0, "right-hand side of `|>` must be a function, call, method"));
    let x = "x |>? 5";
    assert!(reported_at(&err(x), x, "5", 0, "right-hand side of `|>?` must be"));
}

#[test]
fn operator_after_a_stage_is_reported_at_the_operator() {
    for op in ["+", "-", "*", "==", "&&", "||"] {
        let x = format!("x |> f {op} 1");
        let e = err(&x);
        assert!(reported_at(&e, &x, op, 0, "a `|>` stage cannot be followed by an operator"), "{x}: {e}");
    }
    // Parenthesised forms are fine.
    assert!(pipe_ast::sexp("(x |> f) + 1").is_ok());
    assert!(pipe_ast::sexp("x |> (|v| v + 1)").is_ok());
}

#[test]
fn pipe_without_left_hand_value_is_reported_at_the_operator() {
    for op in ["|>", "|>?"] {
        let src = format!("fn t() {{\n    {op} f\n}}\n");
        let e = pipe_ast::parse_program_err(&src).unwrap_err();
        assert!(e.starts_with(&format!("t.wj:2:5: error: `{op}` needs a left-hand value")), "{e}");
    }
}

#[test]
fn multi_line_chain_reports_the_line_of_the_bad_stage() {
    let src = "fn t() {\n    let r = a\n        |> f\n        |> 7\n        |> g\n}\n";
    let e = pipe_ast::parse_program_err(src).unwrap_err();
    assert!(e.starts_with("t.wj:4:12: error: right-hand side of `|>`"), "{e}");
    let src = "fn t() {\n    let r = a\n        |> f(_, b)\n        |> g(_, _)\n}\n";
    let e = pipe_ast::parse_program_err(src).unwrap_err();
    assert!(e.starts_with("t.wj:4:17: error: a pipe stage may contain at most one"), "{e}");
}

#[test]
fn valid_edge_forms_are_accepted() {
    for x in [
        "x |> f(_)",
        "x |> f(a, _)",
        "x |> .m()",
        "x |> m::f(1, _, 2)",
        "x |> T::new(_)",
        "x |> f::<i32>(1)",
        "a |> f(b |> g(_, 1), _)",
        "x |> f()?",
        "x |> f().await",
        "x |>? f |>? g",
    ] {
        assert!(pipe_ast::sexp(x).is_ok(), "{x}: {:?}", pipe_ast::sexp(x));
    }
}

/// Mutation self-test: the span checker must reject a position that is off by one.
#[test]
fn span_checker_rejects_shifted_columns() {
    let x = "x |> f(_, _)";
    let e = err(x);
    assert!(reported_at(&e, x, "_", 1, "a pipe stage"));
    assert!(!reported_at(&e, x, "_", 0, "a pipe stage"), "first `_` must not match");
    assert!(!reported_at(&e, &format!(" {x}"), "_", 1, "a pipe stage"), "shifted source must not match");
}
