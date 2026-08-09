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

// TDD Test: Float literal inference in Vec.push()
//
// Bug: scores.push(0.0) generates 0.0_f64 for Vec<f32>
// Expected: Vec<f32> → push(f32) should constrain argument
//
// Dogfooding Win: Common pattern in game code

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_push_float_literal() {
    let wj_source = r#"
fn init_scores() -> Vec<f32> {
    let mut scores: Vec<f32> = Vec::new()
    scores.push(0.0)
    scores.push(1.0)
    scores.push(2.5)
    scores
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // All literals should be f32 (from Vec<f32> → push(f32))
    assert!(
        !rust_code.contains("_f64"),
        "Float literals should NOT be f64 when pushing to Vec<f32>, got:\n{}",
        rust_code
    );
}

/// Bug (Phase 38 / Graphalytics): `Vec<f64>.push(0.0)` emitted `0.0_f32` (default float).
#[test]
fn test_vec_f64_push_float_literal() {
    let wj_source = r#"
fn init_scores() -> Vec<f64> {
    let mut scores: Vec<f64> = Vec::new()
    scores.push(0.0)
    scores.push(1.0)
    scores
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        !rust_code.contains("0.0_f32") && !rust_code.contains("1.0_f32"),
        "Float literals must be f64 when pushing to Vec<f64>, got:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("0.0_f64") && rust_code.contains("1.0_f64"),
        "Expected 0.0_f64 / 1.0_f64 for Vec<f64>::push, got:\n{}",
        rust_code
    );
}

/// WDB-092: `distances[u] = 0.0` on `Vec<f64>` must emit `0.0_f64`, not default `0.0_f32`.
#[test]
fn test_vec_f64_index_assign_float_literal() {
    let wj_source = r#"
fn seed_source() {
    let mut distances: Vec<f64> = Vec::new()
    distances.push(1.0)
    let u = 0
    distances[u] = 0.0
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        !rust_code.contains("0.0_f32"),
        "Index assign float literal must not be f32 for Vec<f64>, got:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("0.0_f64"),
        "Expected distances[u] = 0.0_f64 for Vec<f64>, got:\n{}",
        rust_code
    );
}

/// WDB-092: comparison against Vec&lt;f64&gt; element must also use f64 literal.
#[test]
fn test_vec_f64_index_compare_float_literal() {
    let wj_source = r#"
fn is_seeded(distances: Vec<f64>, u: int) -> bool {
    distances[u] < 0.0
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        !rust_code.contains("0.0_f32"),
        "Index compare float literal must not be f32 for Vec<f64>, got:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("0.0_f64"),
        "Expected distances[u] < 0.0_f64 for Vec<f64>, got:\n{}",
        rust_code
    );
}
