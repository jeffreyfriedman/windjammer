#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD: if/else used as an expression must not get trailing semicolons on branch values.
//!
//! BUG: `let x: f32 = if c { a } else { b }` wrapped as `Block { If }` was generating
//! `a;` / `b;` inside branches when `in_expression_context` was false and the enclosing
//! function returns `()`, so Rust saw `()` from each branch (E0308).

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_let_binding_if_else_branch_values_no_semicolon() {
    let source = r#"
struct Hud {
    notification_timer: f32,
}

impl Hud {
    fn alpha(self) -> f32 {
        let x: f32 = if self.notification_timer < 1.0 {
            self.notification_timer
        } else {
            1.0
        }
        x
    }
}
"#;

    let code = test_utils::compile_single(source);

    // Regression: missing in_expression_context on `let name: T = rhs` used to emit:
    //   self.notification_timer;
    //   1.0_f32;
    // so the if/else had type `()` instead of `f32`.
    assert!(
        !code.contains("notification_timer;"),
        "then-branch must be a tail expression, not `...;` — generated:\n{code}"
    );
    assert!(
        !code.contains("1.0_f32;") && !code.contains("1.0f32;"),
        "else branch must be a tail float literal, not `...;` — generated:\n{code}"
    );
}

#[test]
fn test_tail_if_else_return_no_semicolon_on_branch_values() {
    let source = r#"
fn pick(t: f32) -> f32 {
    if t < 1.0 {
        t
    } else {
        1.0
    }
}
"#;

    let code = test_utils::compile_single(source);
    assert!(
        code.contains("if t < "),
        "expected comparison if in generated Rust:\n{code}"
    );
    assert!(
        !code.contains("        t;\n"),
        "tail if/else then-branch must not be stmt `t;` — generated:\n{code}"
    );
}

#[test]
fn test_statement_if_else_side_effects_keep_semicolons() {
    let source = r#"
fn bar() {
}

fn baz() {
}

fn foo() {
    if true {
        bar()
    } else {
        baz()
    }
}
"#;

    let code = test_utils::compile_single(source);

    assert!(
        code.contains("bar();"),
        "statement-context if branch should keep semicolon after void call; generated:\n{code}"
    );
    assert!(
        code.contains("baz();"),
        "else branch void call should keep semicolon; generated:\n{code}"
    );
}
