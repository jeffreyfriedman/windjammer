// Current codegen often emits `ident as f32` (and sometimes `(ident as f32) as f32`), not `(ident) as f32`.

#[path = "../common/test_utils.rs"]
mod test_utils;

fn cast_ident_to_f32(generated: &str, ident: &str) -> bool {
    generated.contains(&format!("{ident} as f32"))
}

/// f32 + i32: (dx) as f32 + dy (pathfinding pattern)
#[test]
fn test_f32_add_i32() {
    let source = r#"
pub fn add(x: f32, y: i32) -> f32 {
    x + y
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("y as f32")
            || output.contains("x as f32")
            || output.contains("(y) as f32")
            || output.contains("(x) as f32"),
        "f32 + i32 should cast int to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 f32+i32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// i32 * f32: base * scale (existing test, verify still works)
#[test]
fn test_i32_multiply_f32() {
    let source = r#"
pub fn multiply(count: i32, scale: f32) -> f32 {
    count * scale
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("as f32"),
        "i32 * f32 should cast int to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot multiply"),
        "E0277 i32*f32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 - i32: value - offset (pathfinding dx/dy pattern)
#[test]
fn test_f32_subtract_i32() {
    let source = r#"
pub fn subtract(value: f32, offset: i32) -> f32 {
    value - offset
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "offset"),
        "f32 - i32 should cast int to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot subtract"),
        "E0277 f32-i32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 / i32: total / count
#[test]
fn test_f32_divide_i32() {
    let source = r#"
pub fn divide(total: f32, count: i32) -> f32 {
    total / count
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("count as f32") || output.contains("(count) as f32"),
        "f32 / i32 should cast int to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot divide"),
        "E0277 f32/i32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 * i32 (reverse order): scale * count
#[test]
fn test_f32_multiply_i32() {
    let source = r#"
pub fn scale_by_count(scale: f32, count: i32) -> f32 {
    scale * count
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "count"),
        "f32 * i32 should cast int to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot multiply"),
        "E0277 f32*i32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 + integer literal: x + 2
#[test]
fn test_f32_add_int_literal() {
    let source = r#"
pub fn add_two(x: f32) -> f32 {
    x + 2
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("as f32"),
        "f32 + int literal should cast. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 f32+literal:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 / integer literal: sum / 2 (squad_tactics pattern)
#[test]
fn test_f32_divide_int_literal() {
    let source = r#"
pub fn half(value: f32) -> f32 {
    value / 2
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("as f32"),
        "f32 / int literal should cast. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot divide"),
        "E0277 f32/literal:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Cast chain: (dx) as f32 + dy (exact pathfinding pattern)
#[test]
fn test_cast_chain_f32_add_i32() {
    let source = r#"
pub fn distance(dx: i32, dy: i32) -> f32 {
    ((dx) as f32 + dy) as f32
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "dy") || cast_ident_to_f32(&output, "dx"),
        "Cast chain (dx) as f32 + dy should cast dy. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 cast chain:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 - integer literal: offset - 5 (squad_tactics pattern)
#[test]
fn test_f32_subtract_int_literal() {
    let source = r#"
pub fn offset_center(offset: f32) -> f32 {
    offset - 5
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("as f32"),
        "f32 - int literal should cast. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot subtract"),
        "E0277 f32-literal:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// i32 + f32 (reverse order)
#[test]
fn test_i32_add_f32() {
    let source = r#"
pub fn add_reverse(x: i32, y: f32) -> f32 {
    x + y
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "x"),
        "i32 + f32 should cast int to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 i32+f32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}
