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

/// TDD Test: Mutability Semantics
///
/// ADR-0002: reassigning a `let` without `let mut` is the error WJ0051, one per binding, located at the assignment
/// with the declaration as a second span; with `let mut` the program compiles.
#[path = "common/test_utils.rs"]
mod test_utils;

/// WJ0051 diagnostics for one source: (binding, line of the change, line of the declaration).
fn mut_errors(code: &str) -> Vec<(String, usize, usize)> {
    windjammer::check::local_mutability_sources(&[("test.wj".to_string(), code.to_string())])
        .iter()
        .map(|d| {
            assert!(d.is_error(), "WJ0051 is an error");
            let p = d.primary_span.as_ref().unwrap();
            let s = &d.secondary_spans[0];
            (code[s.byte_start..s.byte_end].to_string(), p.line_start, s.line_start)
        })
        .collect()
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mut_error_message_reassignment() {
    let code = r#"
fn main() {
    let x = 10
    x = 20  // ERROR: cannot assign twice to immutable variable
}
"#;

    assert_eq!(mut_errors(code), [("x".to_string(), 4, 3)]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mut_explicit_compound_assignment() {
    // Immutable-by-default: users must write `let mut` for mutable bindings
    let code = r#"
fn main() {
    let mut count = 0
    count += 1
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Should compile successfully with explicit let mut, got error:\n{:?}",
        result.err()
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mut_explicit_field_mutation() {
    // Immutable-by-default: users must write `let mut` for mutable bindings
    let code = r#"
struct Point {
    pub x: i32,
    pub y: i32,
}

fn main() {
    let mut p = Point { x: 0, y: 0 }
    p.x = 10
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Should compile successfully with explicit let mut, got error:\n{:?}",
        result.err()
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mut_explicit_method_call() {
    // Immutable-by-default: users must write `let mut` for mutable bindings
    let code = r#"
fn main() {
    let mut items = Vec::new()
    items.push(1)
    items.push(2)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Should compile successfully with explicit let mut, got error:\n{:?}",
        result.err()
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mut_works_when_declared() {
    let code = r#"
fn main() {
    let mut x = 10
    x = 20
    x += 5
    println!("{}", x)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Should compile successfully when mut is declared, got error:\n{:?}",
        result.err()
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_multiple_mut_errors() {
    let code = r#"
fn main() {
    let x = 10
    let y = 20
    
    x = 15  // ERROR 1
    y = 25  // ERROR 2
}
"#;

    assert_eq!(mut_errors(code), [("x".to_string(), 6, 3), ("y".to_string(), 7, 4)]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mut_error_with_source_location() {
    let code = r#"
fn main() {
    let x = 10
    x = 20
}
"#;

    assert_eq!(mut_errors(code), [("x".to_string(), 4, 3)]);
}
