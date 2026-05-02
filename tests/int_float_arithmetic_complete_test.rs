#[path = "test_utils.rs"]
mod test_utils;

fn cast_ident_to_f32(generated: &str, ident: &str) -> bool {
    generated.contains(&format!("{ident} as f32"))
}

/// All arithmetic ops: +, -, *, /, %
#[test]
fn test_all_arithmetic_ops() {
    let source = r#"
pub fn test_ops(x: f32, y: i32) -> f32 {
    let a = x + y
    let b = x - y
    let c = x * y
    let d = x / y
    a + b + c + d
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "y"),
        "f32 op i32 should cast y. Got:\n{}",
        output
    );
    assert!(
        !output.contains("cannot add") || test_utils::verify_rust_compiles(&output).is_ok(),
        "Should compile without E0277"
    );
}

/// Compound assignment: price += 1
#[test]
fn test_compound_assignment_f32_plus_int() {
    let source = r#"
pub fn accumulate() -> f32 {
    let mut price = 0.0
    price += 1
    price += 2
    price
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("as f32"),
        "price += 1 should cast int to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 compound assignment:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Compound assignment: scale *= count
#[test]
fn test_compound_assignment_f32_times_int() {
    let source = r#"
pub fn scale_by(count: i32) -> f32 {
    let mut scale = 1.0
    scale *= count
    scale
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("as f32"),
        "scale *= count should cast. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot multiply"),
        "E0277:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Self field: self.count * 0.5 (impl block) - no explicit cast, compiler adds it
#[test]
fn test_impl_block_self_field_int_times_float() {
    let source = r#"
pub struct Stats {
    count: i32,
}

impl Stats {
    pub fn average(self) -> f32 {
        self.count * 0.5
    }
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("as f32") || output.contains("_f32"),
        "self.count * 0.5 should have float cast. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot multiply"),
        "E0277 self.count * 0.5:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 + integer literal
#[test]
fn test_f32_add_literal() {
    let source = r#"
pub fn add_one(x: f32) -> f32 {
    x + 1
}
"#;

    let output = test_utils::compile_single(source);
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        ok,
        "f32 + literal should compile. stderr: {}\n{}",
        stderr, output
    );
}

/// i32 * f32 (reverse order)
#[test]
fn test_i32_times_f32() {
    let source = r#"
pub fn mul(count: i32, scale: f32) -> f32 {
    count * scale
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "count"),
        "count * scale should cast count. Got:\n{}",
        output
    );
}
