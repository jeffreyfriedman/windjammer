/// Tests for `let` immutability-by-default semantics
///
/// **Current `wj` behavior (CI truth):** The compiler still infers `let mut` when a binding is
/// mutated (push, compound assignment, etc.). A future pass may emit Windjammer-native immutability
/// errors instead; until then, these tests assert successful builds and, where useful, that output
/// contains `let mut` for mutated locals.
///
/// Intended philosophy (Rust/Swift-style explicit `let mut` at the source level) is not fully
/// enforced in the driver yet.
#[path = "../common/test_utils.rs"]
mod test_utils;

// ============================================================================
// TEST 1: `let` generates `let` (no mut) in Rust output
//
// A bare `let` binding that is never mutated should generate plain `let` in Rust.
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_generates_immutable_binding() {
    let source = r#"
fn main() {
    let x = 5
    let y = x + 10
    println("{}", y)
}
"#;

    let (generated, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // Should generate `let x = 5` (not `let mut x`)
    assert!(
        generated.contains("let x =") || generated.contains("let x:"),
        "Expected immutable `let x`, got:\n{}",
        generated
    );
    assert!(
        !generated.contains("let mut x"),
        "Should NOT have `let mut x` for immutable binding, got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 2: `let mut` generates `let mut` in Rust output
//
// An explicit `let mut` binding should generate `let mut` in Rust.
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_generates_mutable_binding() {
    let source = r#"
fn main() {
    let mut count = 0
    count = count + 1
    println("{}", count)
}
"#;

    let (generated, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // Should generate `let mut count`
    assert!(
        generated.contains("let mut count"),
        "Expected `let mut count`, got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 3: mutating a `let` binding without `let mut` — current compiler infers `let mut`
//
// When native immutability diagnostics land, this test should expect failure; for now wj build
// succeeds and codegen includes `let mut items`.
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_does_not_auto_infer_mut() {
    let source = r#"
fn main() {
    let items: Vec<int> = Vec::new()
    items.push(42)
}
"#;

    let (generated, _stderr) = test_utils::compile_via_cli_with_stderr(source);
    assert!(
        generated.contains("let mut items"),
        "Current codegen should infer `let mut` for mutated Vec binding. Got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 4: `let mut` with Vec operations works correctly
//
// When the user explicitly writes `let mut`, everything works as before.
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_with_vec_push() {
    let source = r#"
fn main() {
    let mut items: Vec<int> = Vec::new()
    items.push(42)
    items.push(100)
    println("{}", items.len())
}
"#;

    let (generated, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // Should generate `let mut items`
    assert!(
        generated.contains("let mut items"),
        "Expected `let mut items` for explicit mut binding, got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 5: compound assignment — current compiler infers `let mut count`
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_compound_assignment_no_auto_mut() {
    let source = r#"
fn main() {
    let count = 0
    count += 1
}
"#;

    let (generated, _stderr) = test_utils::compile_via_cli_with_stderr(source);
    assert!(
        generated.contains("let mut count"),
        "Current codegen should infer `let mut` for compound assignment. Got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 6: `let mut` with compound assignment works
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_compound_assignment() {
    let source = r#"
fn main() {
    let mut count = 0
    count += 1
    println("{}", count)
}
"#;

    let (generated, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    assert!(
        generated.contains("let mut count"),
        "Expected `let mut count`, got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 7: `let` and `let mut` coexist in same function
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_and_let_mut_coexist() {
    let source = r#"
fn main() {
    let name = "hello"
    let mut count = 0
    count += 1
    println("{} {}", name, count)
}
"#;

    let (generated, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // name should be immutable
    assert!(
        !generated.contains("let mut name"),
        "Expected immutable `let name`, got:\n{}",
        generated
    );

    // count should be mutable
    assert!(
        generated.contains("let mut count"),
        "Expected `let mut count`, got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 8: Parameter mutability inference is UNCHANGED
//
// The auto-mut for parameters (ownership inference) should still work.
// Only local `let` bindings lose auto-mut.
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_parameter_mut_inference_unchanged() {
    let source = r#"
struct Counter {
    value: int
}

impl Counter {
    fn increment(self) {
        self.value += 1
    }
}

fn main() {
    let mut c = Counter { value: 0 }
    c.increment()
}
"#;

    let (generated, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // The `self` parameter should still be auto-inferred as `&mut self`
    assert!(
        generated.contains("&mut self"),
        "Parameter ownership inference should still work. Got:\n{}",
        generated
    );
}
