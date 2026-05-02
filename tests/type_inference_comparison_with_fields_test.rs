/// TDD Test: Field access in comparison operations
///
/// Bug: self.current_wait > 0.0 generates 0.0_f64 instead of 0.0_f32
/// Root Cause: Field type not propagating through comparison operators
/// Expected: Comparison operands should match field type
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_field_in_comparison_gt() {
    let source = r#"
struct Timer {
    current: f32,
}

impl Timer {
    pub fn is_active() -> bool {
        self.current > 0.0
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // The 0.0 literal should be f32 (matches field type)
    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32' (to match self.current: f32), got: {}",
        output
    );
    assert!(
        !output.contains("0.0_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

#[test]
fn test_field_in_comparison_lt() {
    let source = r#"
struct Position {
    x: f32,
    y: f32,
}

impl Position {
    pub fn is_left_of_origin() -> bool {
        self.x < 0.0
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32', got: {}",
        output
    );
}

#[test]
fn test_field_in_comparison_eq() {
    let source = r#"
struct Health {
    value: f32,
}

impl Health {
    pub fn is_zero() -> bool {
        self.value == 0.0
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32', got: {}",
        output
    );
}

#[test]
fn test_field_in_comparison_with_param() {
    let source = r#"
struct Cooldown {
    remaining: f32,
}

impl Cooldown {
    pub fn update(dt: f32) -> bool {
        if self.remaining > 0.0 {
            self.remaining = self.remaining - dt
            return false
        }
        return true
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Both 0.0 should be f32
    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32', got: {}",
        output
    );
    assert!(
        !output.contains("0.0_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
