// Explicit cast + integer literal: `x as f32 + 1` should cast `1` to f32 too

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_explicit_cast_plus_int_literal() {
    let source = r#"
pub fn compute(x: i32) -> f32 {
    x as f32 + 1
}
"#;

    let result = test_utils::compile_single(source);

    let __result = test_utils::verify_rust_compiles(&result);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok,
        "`x as f32 + 1` should auto-cast literal to f32. stderr: {}\n\nGenerated:\n{}",
        stderr, result
    );
}

/// usize + f32 field: index + position.x should auto-cast index to f32
#[test]
fn test_usize_plus_f32_field() {
    let source = r#"
pub struct Camera {
    offset_x: f32,
}

impl Camera {
    pub fn screen_x(self, tile_index: usize) -> f32 {
        tile_index + self.offset_x
    }
}
"#;

    let result = test_utils::compile_single(source);

    let __result = test_utils::verify_rust_compiles(&result);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok,
        "`usize + f32` should auto-cast usize to f32. stderr: {}\n\nGenerated:\n{}",
        stderr, result
    );
}

/// Function call: i32 args passed to f32 parameters - auto-cast at call site
#[test]
fn test_function_call_i32_to_f32_args() {
    let source = r#"
pub struct Grid {
    width: i32,
    height: i32,
}

impl Grid {
    pub fn is_walkable(self, x: f32, y: f32) -> bool {
        x >= 0.0 && y >= 0.0
    }

    pub fn get_neighbors(self, x: i32, y: i32) -> Vec<bool> {
        let mut result = Vec::new()
        result.push(self.is_walkable(x + 1, y))
        result.push(self.is_walkable(x, y + 1))
        result.push(self.is_walkable(x - 1, y))
        result.push(self.is_walkable(x, y - 1))
        result
    }
}
"#;

    let result = test_utils::compile_single(source);

    let __result = test_utils::verify_rust_compiles(&result);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok,
        "Function call should auto-cast i32 args to f32. stderr: {}\n\nGenerated:\n{}",
        stderr, result
    );
}

/// Mixed multiplication: i32 * f32
#[test]
fn test_mixed_int_float_multiplication() {
    let source = r#"
pub struct Physics {
    speed: f32,
}

impl Physics {
    pub fn compute(self, frames: i32) -> f32 {
        let distance = frames * self.speed
        distance
    }
}
"#;

    let result = test_utils::compile_single(source);

    let __result = test_utils::verify_rust_compiles(&result);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok,
        "i32 * f32 should auto-cast. stderr: {}\n\nGenerated:\n{}",
        stderr, result
    );
}

/// Function call with i32 arg where f32 param expected — auto-cast at call site
#[test]
fn test_function_call_int_to_float_autocast() {
    let source = r#"
pub struct Grid {
    width: i32,
}

impl Grid {
    pub fn get_position(self, index: i32) -> f32 {
        index * 10.0
    }
}
"#;

    let result = test_utils::compile_single(source);

    let __result = test_utils::verify_rust_compiles(&result);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok,
        "i32 * f32_literal should auto-cast. stderr: {}\n\nGenerated:\n{}",
        stderr, result
    );
}
