#[path = "test_utils.rs"]
mod test_utils;

fn cast_ident_to_f32(generated: &str, ident: &str) -> bool {
    generated.contains(&format!("{ident} as f32"))
}

/// True if stderr indicates E0277 (trait/type) - our codegen bug.
/// False for infrastructure errors (temp file, memory map, etc.).
fn is_e0277_codegen_error(stderr: &str) -> bool {
    stderr.contains("E0277")
        && (stderr.contains("cannot add")
            || stderr.contains("cannot subtract")
            || stderr.contains("cannot multiply")
            || stderr.contains("cannot divide")
            || stderr.contains("no implementation for"))
}

/// f32 % i32 → should auto-cast i32 to f32
#[test]
fn test_f32_mod_i32() {
    let source = r#"
pub fn wrap(value: f32, count: i32) -> f32 {
    value % count
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "count"),
        "f32 % i32 should cast count to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// f32 + i32 → should auto-cast i32 to f32
#[test]
fn test_f32_add_i32() {
    let source = r#"
pub fn add(value: f32, count: i32) -> f32 {
    value + count
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "count"),
        "f32 + i32 should cast count to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// f32 * i32 → should auto-cast i32 to f32
#[test]
fn test_f32_multiply_i32() {
    let source = r#"
pub fn scale(value: f32, count: i32) -> f32 {
    value * count
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "count"),
        "f32 * i32 should cast count to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// (i32 % i32) as f32 - must NOT cast operands; outer cast handles f32.
/// Regression: tilemap.wj (sprite_index % tiles_per_row) as f32 was generating
/// (sprite_index) as f32 % tiles_per_row (wrong - f32 % i32).
#[test]
fn test_int_mod_int_then_cast_not_operands() {
    let source = r#"
pub struct Tile {
    pub sprite_index: i32,
}

pub fn uv_coord(tile: Tile, tiles_per_row: i32) -> f32 {
    (tile.sprite_index % tiles_per_row) as f32 / tiles_per_row as f32
}
"#;

    let output = test_utils::compile_single(source);
    // Must have (sprite_index % tiles_per_row) as f32, NOT (sprite_index) as f32 % tiles_per_row
    assert!(
        output.contains("sprite_index % tiles_per_row) as f32"),
        "Should cast result of int%%int: (sprite_index % tiles_per_row) as f32. Got:\n{}",
        output
    );
    assert!(
        !output.contains("sprite_index) as f32 % tiles_per_row"),
        "Must NOT cast left operand of int%%int (would produce f32%%i32). Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// f32 / i32 → should auto-cast i32 to f32
#[test]
fn test_f32_divide_i32() {
    let source = r#"
pub fn divide(value: f32, count: i32) -> f32 {
    value / count
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "count"),
        "f32 / i32 should cast count to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// f32 - i32 → should auto-cast i32 to f32
#[test]
fn test_f32_subtract_i32() {
    let source = r#"
pub fn subtract(value: f32, count: i32) -> f32 {
    value - count
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        cast_ident_to_f32(&output, "count"),
        "f32 - i32 should cast count to f32. Got:\n{}",
        output
    );

    let __result = test_utils::verify_rust_compiles(&output);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}
