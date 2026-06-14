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

/// TDD: WGSL-style if-expressions used as values must type-check.
///
/// Bug: `let x = if (cond) { a } else { b }` fails with
/// "[line N:18] Unexpected token in expression: If" because the WJSL
/// body type checker only supported if *statements*, not if *expressions*.
use windjammer::wjsl::transpile_wjsl;

#[test]
fn test_if_expression_in_let_binding() {
    let source = r#"
@compute @workgroup_size(8, 8)
fn main() {
    let drill = 1.0;
    let panel_h = if (drill > 0.5) { 100u } else { 50u };
}
"#;
    let wgsl = transpile_wjsl(source).expect("if-expression in let binding should transpile");
    assert!(wgsl.contains("select(50u, 100u, drill > 0.5)"));
}

#[test]
fn test_if_expression_multiline_branches() {
    let source = r#"
@compute @workgroup_size(8, 8)
fn main() {
    let budget = 100.0;
    let ratio = if (budget > 0.0) {
        50.0 / budget
    } else {
        0.0
    };
}
"#;
    transpile_wjsl(source).expect("multiline if-expression branches should transpile");
}

#[test]
fn test_if_expression_lowers_to_select_in_wgsl() {
    let source = r#"
@compute @workgroup_size(8, 8)
fn main() {
    let drill = 1.0;
    let panel_h = if (drill > 0.5) { 100u } else { 50u };
}
"#;
    let wgsl = transpile_wjsl(source).expect("should transpile");
    assert!(
        wgsl.contains("select(50u, 100u, drill > 0.5)"),
        "WGSL output should lower if-expression to select(): {}",
        wgsl
    );
    assert!(
        !wgsl.contains("if (drill > 0.5) { 100u }"),
        "WGSL should not contain raw if-expression"
    );
}

#[test]
fn test_if_expression_nested_in_call() {
    let source = r#"
fn pick(v: f32) -> f32 { return v; }

@compute @workgroup_size(8, 8)
fn main() {
    let fps = 30.0;
    let color = pick(if (fps >= 60.0) { 0.5 } else { 1.1 });
}
"#;
    transpile_wjsl(source).expect("nested if-expression in call arg should transpile");
}
