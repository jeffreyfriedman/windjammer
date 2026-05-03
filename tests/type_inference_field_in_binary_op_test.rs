/// TDD Test: Float inference for struct field in binary operations
///
/// Bug: self.vy * 0.5 where vy: f32 generates 0.5_f64 instead of 0.5_f32
/// Pattern: FieldAccess is used in binary operation with float literal
/// Expected: Literal should be constrained to field's type (f32)
///
/// Example from windjammer-game:
/// ```windjammer
/// struct CharacterController {
///     pub vy: f32, // velocity Y
/// }
/// impl CharacterController {
///     pub fn release_jump(self) {
///         self.vy = self.vy * 0.5  // Literal should be f32, not f64
///     }
/// }
/// ```
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_field_in_binary_op_simple() {
    let source = r#"pub struct Controller {
    pub vy: f32,
}

impl Controller {
    pub fn half_speed(self) {
        self.vy = self.vy * 0.5
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Should generate 0.5_f32 since vy is f32
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32' in generated code, got:\n{}",
        output
    );

    // Should NOT generate 0.5_f64
    assert!(
        !output.contains("0.5_f64"),
        "Should not contain '0.5_f64', but it does:\n{}",
        output
    );
}

#[test]
fn test_field_in_binary_op_complex() {
    let source = r#"pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn scale(self, factor: f32) -> Vec3 {
        Vec3 {
            x: self.x * factor * 0.5,
            y: self.y * factor * 0.25,
            z: self.z * factor,
        }
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // All literals should be f32
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32' in generated code"
    );
    assert!(
        output.contains("0.25_f32"),
        "Expected '0.25_f32' in generated code"
    );

    // Should NOT generate any f64
    assert!(
        !output.contains("_f64"),
        "Should not contain any '_f64' literals:\n{}",
        output
    );
}

#[test]
fn test_field_in_nested_binary_ops() {
    let source = r#"pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn transform(self) -> Point {
        Point {
            x: (self.x * 2.0 + 10.0) * 0.5,
            y: (self.y * 2.0 + 10.0) * 0.5,
        }
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // All literals should be f32
    assert!(
        output.contains("2.0_f32"),
        "Expected '2.0_f32' in generated code"
    );
    assert!(
        output.contains("10.0_f32"),
        "Expected '10.0_f32' in generated code"
    );
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32' in generated code"
    );

    // Should NOT generate any f64
    assert!(
        !output.contains("_f64"),
        "Should not contain any '_f64' literals:\n{}",
        output
    );
}

// Helper function to compile Windjammer source and get generated Rust
