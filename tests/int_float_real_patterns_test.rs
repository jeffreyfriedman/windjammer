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

// Pattern: member_index as f32 * 6.28318 / count as f32 (ai/squad_tactics Circle formation)

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_squad_tactics_formation_angle() {
    let source = r#"
pub fn formation_angle(member_index: i32, count: i32) -> f32 {
    member_index as f32 * 6.28318 / count as f32
}
"#;

    let output = test_utils::compile_single(source);
    let has_f32_safety = output.contains("as f32") || output.contains("_f32");
    assert!(
        has_f32_safety,
        "Squad tactics pattern should have f32 consistency. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot multiply") && !stderr.contains("cannot divide"),
        "E0277 squad_tactics:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Pattern: (seed * 1234.567).sin() * 3.14159265 * 2.0 (particles/emitter)
#[test]
fn test_emitter_angle_chain() {
    let source = r#"
pub fn emit_angle(seed: f32) -> f32 {
    (seed * 1234.567).sin() * 3.14159265 * 2.0
}
"#;

    let output = test_utils::compile_single(source);
    let has_f32_safety = output.contains("as f32") || output.contains("_f32");
    assert!(
        has_f32_safety,
        "Emitter pattern should have f32 consistency. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot multiply") && !stderr.contains("cannot add"),
        "E0277 emitter:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Pattern: f32 + i32 (calculate_cost from pathfinding)
#[test]
fn test_calculate_cost_f32_add_i32() {
    let source = r#"
pub fn calculate_cost(base: f32, penalty: i32) -> f32 {
    base + penalty
}
"#;

    let output = test_utils::compile_single(source);
    // Codegen may emit `(penalty as f32) as f32` (redundant but valid) or `(penalty) as f32`
    assert!(
        output.contains("penalty as f32")
            || output.contains("(penalty) as f32")
            || output.contains("(base) as f32"),
        "base + penalty should cast int to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 calculate_cost:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Pattern: f32 += i32 (compound assignment)
#[test]
fn test_compound_f32_add_assign_i32() {
    let source = r#"
pub fn accumulate(total: f32, value: i32) -> f32 {
    let mut t = total;
    t += value;
    t
}
"#;

    let output = test_utils::compile_single(source);
    let has_cast = output.contains("as f32");
    assert!(
        has_cast,
        "t += value should cast value to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 compound assign:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}
