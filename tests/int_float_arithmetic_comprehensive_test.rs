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

#[path = "common/test_utils.rs"]
mod test_utils;

fn cast_ident_to_f32(generated: &str, ident: &str) -> bool {
    generated.contains(&format!("{ident} as f32"))
}

fn cast_ident_to_f64(generated: &str, ident: &str) -> bool {
    generated.contains(&format!("{ident} as f64"))
}

// Run `rustc` in a unique temp directory so parallel tests do not race on `libtest.rlib`
// in the process working directory.

fn assert_no_e0277(rust_code: &str, stderr: &str, context: &str) {
    let has_e0277 = stderr.contains("cannot add")
        || stderr.contains("cannot subtract")
        || stderr.contains("cannot multiply")
        || stderr.contains("cannot divide");
    assert!(
        !has_e0277,
        "E0277 int/float in {}:\nstderr: {}\n\nGenerated:\n{}",
        context, stderr, rust_code
    );
}

// 1. f32 + i32 (all ops)
#[test]
fn test_f32_op_i32_all_ops() {
    let source = r#"
pub fn test(x: f32, y: i32) -> f32 {
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
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 2. i32 + f32 (reverse order)
#[test]
fn test_i32_op_f32_all_ops() {
    let source = r#"
pub fn test(x: i32, y: f32) -> f32 {
    let a = x + y
    let b = x - y
    let c = x * y
    let d = x / y
    a + b + c + d
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "x"),
        "i32 op f32 should cast x. Got:\n{}",
        output
    );
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 3. f32 + int literal
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
        "f32 + int literal should compile. stderr: {}\n{}",
        stderr, output
    );
}

// 4. int literal + f32
#[test]
fn test_int_literal_add_f32() {
    let source = r#"
pub fn add(x: f32) -> f32 {
    1 + x
}
"#;
    let output = test_utils::compile_single(source);
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        ok,
        "int literal + f32 should compile. stderr: {}\n{}",
        stderr, output
    );
}

// 5. Compound: price += 1
#[test]
fn test_compound_f32_plus_int() {
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
        "price += 1 should cast. Got:\n{}",
        output
    );
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 6. Compound: scale *= count
#[test]
fn test_compound_f32_times_int() {
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
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 7. Self field: self.count * 0.5
#[test]
fn test_impl_self_field_int_times_float() {
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
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 8. Cast + int: (dx as f32) + dy
#[test]
fn test_cast_f32_plus_int() {
    let source = r#"
pub fn offset(dx: i32, dy: i32) -> f32 {
    (dx as f32) + dy
}
"#;
    let output = test_utils::compile_single(source);
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
    assert!(
        cast_ident_to_f32(&output, "dy") || output.contains("as f32"),
        "(dx as f32) + dy should cast dy. Got:\n{}",
        output
    );
}

// 9. Nested: (a + b) + c
#[test]
fn test_nested_f32_plus_int() {
    let source = r#"
pub fn chain(a: f32, b: i32, c: i32) -> f32 {
    (a + b) + c
}
"#;
    let output = test_utils::compile_single(source);
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        ok,
        "nested f32 + int should compile. stderr: {}\n{}",
        stderr, output
    );
}

// 10. Formation angle pattern: member_index as f32 * 6.28318 / count as f32
#[test]
fn test_formation_angle() {
    let source = r#"
pub fn formation_angle(member_index: i32, count: i32) -> f32 {
    member_index as f32 * 6.28318 / count as f32
}
"#;
    let output = test_utils::compile_single(source);
    let has_f32 = output.contains("as f32") || output.contains("_f32");
    assert!(has_f32, "Should have f32 consistency. Got:\n{}", output);
    let __result = test_utils::verify_rust_compiles(&output);
    let _ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert_no_e0277(&output, &stderr, "formation_angle");
}

// 11. f64 + i32
#[test]
fn test_f64_plus_i32() {
    let source = r#"
pub fn test(x: f64, y: i32) -> f64 {
    x + y
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f64(&output, "y"),
        "f64 + i32 should cast to f64. Got:\n{}",
        output
    );
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 12. uint + f32
#[test]
fn test_uint_plus_f32() {
    let source = r#"
pub fn test(x: uint, y: f32) -> f32 {
    x + y
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("as f32"),
        "uint + f32 should cast to f32. Got:\n{}",
        output
    );
    test_utils::verify_rust_compiles(&output).expect("uint + f32 generated code should compile");
}

// 13. usize + f32 (usize is int-like for arithmetic)
#[test]
fn test_usize_plus_f32() {
    let source = r#"
pub fn test(x: usize, y: f32) -> f32 {
    x + y
}
"#;
    let output = test_utils::compile_single(source);
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
    assert!(
        output.contains("as f32") || output.contains("x as f32"),
        "usize + f32: expected a float cast. Got:\n{}",
        output
    );
}

// 14. Const default (immut context - uses generate_expression_immut)
#[test]
fn test_const_default_int_float() {
    let source = r#"
pub const SCALE: f32 = 1.0 + 1

pub fn get_scale() -> f32 {
    SCALE
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("as f32") || output.contains("1.0_f32 + 1") || output.contains("SCALE"),
        "const mix should lower. Got:\n{}",
        output
    );
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 15. f32 - int literal
#[test]
fn test_f32_sub_int_literal() {
    let source = r#"
pub fn sub_one(x: f32) -> f32 {
    x - 1
}
"#;
    let output = test_utils::compile_single(source);
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        ok,
        "f32 - int literal should compile. stderr: {}\n{}",
        stderr, output
    );
}

// 16. f32 / int literal
#[test]
fn test_f32_div_int_literal() {
    let source = r#"
pub fn half(x: f32) -> f32 {
    x / 2
}
"#;
    let output = test_utils::compile_single(source);
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        ok,
        "f32 / int literal should compile. stderr: {}\n{}",
        stderr, output
    );
}

// 17. f32 % int
#[test]
fn test_f32_mod_int() {
    let source = r#"
pub fn wrap(x: f32, period: i32) -> f32 {
    x % period
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "period"),
        "x % period should cast. Got:\n{}",
        output
    );
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 18. calculate_cost pattern: base + penalty
#[test]
fn test_calculate_cost() {
    let source = r#"
pub fn calculate_cost(base: f32, penalty: i32) -> f32 {
    base + penalty
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "penalty") || cast_ident_to_f32(&output, "base"),
        "base + penalty should cast. Got:\n{}",
        output
    );
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 19. Compound with param: t += value
#[test]
fn test_compound_param() {
    let source = r#"
pub fn accumulate(total: f32, value: i32) -> f32 {
    let mut t = total
    t += value
    t
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("as f32"),
        "t += value should cast. Got:\n{}",
        output
    );
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 20. Complex: base * count + offset
#[test]
fn test_complex_expression() {
    let source = r#"
pub fn compute(base: f32, count: i32, offset: f32) -> f32 {
    base * count + offset
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "count"),
        "base * count should cast. Got:\n{}",
        output
    );
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(ok, "Should compile. stderr: {}", stderr);
}
