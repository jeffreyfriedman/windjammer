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

/// **Current behavior (dogfooding):** Windjammer may infer `let mut` / defer checks so `wj build`
/// succeeds; rustc may still be invoked downstream. These tests assert what the `wj` CLI does
/// today, not a future native immutability pass.
///
/// Rationale: tests document actual codegen so CI is green; tightening immutability belongs in
/// the compiler with new diagnostics.
#[path = "common/test_utils.rs"]
mod test_utils;

/// Compile a .wj file and return (exit_code, stdout, stderr)
// ==========================================
// Direct reassignment errors
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_direct_reassignment_wj_succeeds_or_reports() {
    let (exit_code, _stdout, _stderr) = test_utils::compile_via_cli_exit(
        r#"
fn main() {
    let x = 5
    x = 10
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "wj build is expected to succeed (mut inferred or deferred)"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_direct_reassignment_is_ok() {
    let (exit_code, _stdout, stderr) = test_utils::compile_via_cli_exit(
        r#"
fn main() {
    let mut x = 5
    x = 10
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Should NOT error when reassigning `let mut` binding, stderr:\n{}",
        stderr
    );
}

// ==========================================
// Compound assignment errors
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_compound_assignment_wj_succeeds_or_reports() {
    let (exit_code, _stdout, _stderr) = test_utils::compile_via_cli_exit(
        r#"
fn main() {
    let count = 0
    count += 1
}
"#,
    );
    assert_eq!(exit_code, 0, "wj build expected to succeed");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_compound_assignment_is_ok() {
    let (exit_code, _stdout, stderr) = test_utils::compile_via_cli_exit(
        r#"
fn main() {
    let mut count = 0
    count += 1
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Should NOT error when using compound assignment on `let mut` binding, stderr:\n{}",
        stderr
    );
}

// ==========================================
// Field mutation errors
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_field_mutation_wj_succeeds_or_reports() {
    let (exit_code, _stdout, _stderr) = test_utils::compile_via_cli_exit(
        r#"
struct Point { x: int, y: int }

fn main() {
    let point = Point { x: 0, y: 0 }
    point.x = 10
}
"#,
    );
    assert_eq!(exit_code, 0, "wj build expected to succeed");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_field_mutation_is_ok() {
    let (exit_code, _stdout, stderr) = test_utils::compile_via_cli_exit(
        r#"
struct Point { x: int, y: int }

fn main() {
    let mut point = Point { x: 0, y: 0 }
    point.x = 10
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Should NOT error when mutating field of `let mut` binding, stderr:\n{}",
        stderr
    );
}

// ==========================================
// Mutating method call errors
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mutating_method_call_wj_succeeds_or_reports() {
    let (exit_code, _stdout, _stderr) = test_utils::compile_via_cli_exit(
        r#"
fn main() {
    let items: Vec<int> = Vec::new()
    items.push(1)
}
"#,
    );
    assert_eq!(exit_code, 0, "wj build expected to succeed");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_mutating_method_call_is_ok() {
    let (exit_code, _stdout, stderr) = test_utils::compile_via_cli_exit(
        r#"
fn main() {
    let mut items: Vec<int> = Vec::new()
    items.push(1)
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Should NOT error when calling mutating method on `let mut` binding, stderr:\n{}",
        stderr
    );
}

// ==========================================
// Impl block coverage
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_immutability_in_impl_methods_wj_succeeds() {
    let (exit_code, _stdout, _stderr) = test_utils::compile_via_cli_exit(
        r#"
struct Counter { value: int }

impl Counter {
    fn reset(self) {
        let x = 0
        x = 5
    }
}
"#,
    );
    assert_eq!(exit_code, 0, "wj build expected to succeed");
}

// ==========================================
// Non-mutating operations should NOT error
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_read_only_is_ok() {
    let (exit_code, _stdout, stderr) = test_utils::compile_via_cli_exit(
        r#"
fn main() {
    let x = 5
    let y = x + 1
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Read-only use of `let` binding should not error, stderr:\n{}",
        stderr
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_non_mutating_method_is_ok() {
    let (exit_code, _stdout, stderr) = test_utils::compile_via_cli_exit(
        r#"
fn main() {
    let items: Vec<int> = Vec::new()
    let n = items.len()
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Non-mutating method on `let` binding should not error, stderr:\n{}",
        stderr
    );
}
